use crate::domain::workspace::WorkspaceContext;
use crate::infrastructure::legacy_scripts::{find_plugin_root, value_to_cli_string};
use crate::infrastructure::AdapterOutcome;
use serde_json::{json, Map, Value};
use std::env;
use std::path::PathBuf;
use std::process::Command;

pub struct CliAdapter {
    launcher: &'static str,
    default_command: &'static [&'static str],
    label: &'static str,
}

impl CliAdapter {
    pub fn new(
        launcher: &'static str,
        default_command: &'static [&'static str],
        label: &'static str,
    ) -> Self {
        Self {
            launcher,
            default_command,
            label,
        }
    }

    pub fn invoke(
        &self,
        tool_name: &str,
        args: &Map<String, Value>,
        context: &WorkspaceContext,
        dry_run: bool,
        mutating: bool,
    ) -> Result<AdapterOutcome, String> {
        let plugin_root = find_plugin_root(&context.cwd).ok_or_else(|| {
            "could not locate Unica plugin root for internal adapter lookup".to_string()
        })?;
        let launcher = plugin_root.join("scripts").join(self.launcher);
        let mut command = vec![launcher.display().to_string()];
        command.extend(self.default_command.iter().map(|part| (*part).to_string()));
        command.extend(cli_args(args));

        if dry_run {
            return Ok(AdapterOutcome {
                ok: true,
                summary: format!(
                    "dry run: {tool_name} would call internal {} adapter",
                    self.label
                ),
                changes: if mutating {
                    vec!["no files changed because dryRun is true".to_string()]
                } else {
                    Vec::new()
                },
                warnings: if launcher.exists() {
                    Vec::new()
                } else {
                    vec![format!(
                        "internal adapter launcher not found: {}",
                        launcher.display()
                    )]
                },
                errors: Vec::new(),
                artifacts: Vec::new(),
                stdout: None,
                stderr: None,
                command: Some(command),
            });
        }

        if !launcher.exists() {
            return Err(format!(
                "internal adapter launcher not found: {}",
                launcher.display()
            ));
        }

        let output = Command::new(&launcher)
            .args(self.default_command)
            .args(cli_args(args))
            .current_dir(&context.cwd)
            .output()
            .map_err(|err| format!("failed to execute internal {} adapter: {err}", self.label))?;
        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        let ok = output.status.success();
        Ok(AdapterOutcome {
            ok,
            summary: if ok {
                format!(
                    "{tool_name} completed through internal {} adapter",
                    self.label
                )
            } else {
                format!("{tool_name} failed through internal {} adapter", self.label)
            },
            changes: if mutating {
                vec![format!("internal {} adapter executed", self.label)]
            } else {
                Vec::new()
            },
            warnings: if ok {
                Vec::new()
            } else {
                vec![format!(
                    "internal {} adapter exited with status {}",
                    self.label, output.status
                )]
            },
            errors: if ok {
                Vec::new()
            } else {
                vec![stderr.trim().to_string()]
            },
            artifacts: Vec::new(),
            stdout: Some(stdout),
            stderr: Some(stderr),
            command: Some(command),
        })
    }
}

pub struct StandardsAdapter;

#[derive(Debug, Clone, PartialEq)]
pub struct StandardsRequest {
    pub method: &'static str,
    pub params: Value,
}

impl StandardsAdapter {
    pub fn request_for(
        operation: &str,
        args: &Map<String, Value>,
    ) -> Result<StandardsRequest, String> {
        match operation {
            "search" => Ok(StandardsRequest {
                method: "v8std_search",
                params: select_params(args, &["query", "limit", "types", "mode"]),
            }),
            "explain" if args.contains_key("codes") => Ok(StandardsRequest {
                method: "v8std_explain_diagnostics",
                params: select_params(args, &["codes"]),
            }),
            "explain" if args.contains_key("snippet") => Ok(StandardsRequest {
                method: "v8std_explain_snippet",
                params: select_params(args, &["snippet", "language", "limit"]),
            }),
            "explain" if args.contains_key("id") || args.contains_key("idOrAliasOrUrl") => {
                let id = args
                    .get("idOrAliasOrUrl")
                    .or_else(|| args.get("id"))
                    .cloned()
                    .ok_or_else(|| "missing id".to_string())?;
                let mut params = Map::new();
                params.insert("id_or_alias_or_url".to_string(), id);
                if let Some(limit) = args.get("bodyLimit").or_else(|| args.get("body_limit")) {
                    params.insert("body_limit".to_string(), limit.clone());
                }
                Ok(StandardsRequest {
                    method: "v8std_get_page",
                    params: Value::Object(params),
                })
            }
            "explain" if args.contains_key("query") => Ok(StandardsRequest {
                method: "v8std_search",
                params: select_params(args, &["query", "limit", "types", "mode"]),
            }),
            "explain" => Err(
                "unica.standards.explain requires one of: codes, snippet, id, idOrAliasOrUrl, query"
                    .to_string(),
            ),
            other => Err(format!("unknown standards operation: {other}")),
        }
    }

    pub fn invoke(operation: &str, args: &Map<String, Value>) -> AdapterOutcome {
        let endpoint = env::var("UNICA_STANDARDS_MCP_URL")
            .unwrap_or_else(|_| "https://ai.v8std.ru/mcp".to_string());
        let request = match Self::request_for(operation, args) {
            Ok(request) => request,
            Err(error) => {
                return AdapterOutcome {
                    ok: false,
                    summary: format!("unica.standards.{operation} rejected invalid arguments"),
                    changes: Vec::new(),
                    warnings: Vec::new(),
                    errors: vec![error],
                    artifacts: vec![endpoint],
                    stdout: None,
                    stderr: None,
                    command: None,
                }
            }
        };

        let payload = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": request.method,
                "arguments": request.params,
            }
        });

        match ureq::post(&endpoint)
            .set("Content-Type", "application/json")
            .set("Accept", "application/json, text/event-stream")
            .send_string(&payload.to_string())
        {
            Ok(response) => match response.into_string() {
                Ok(text) => {
                    let normalized = normalize_mcp_http_body(&text);
                    AdapterOutcome {
                        ok: true,
                        summary: format!(
                            "unica.standards.{operation} completed through internal v8std MCP proxy"
                        ),
                        changes: Vec::new(),
                        warnings: Vec::new(),
                        errors: Vec::new(),
                        artifacts: vec![endpoint, request.method.to_string()],
                        stdout: Some(normalized),
                        stderr: None,
                        command: None,
                    }
                }
                Err(err) => AdapterOutcome {
                    ok: false,
                    summary: format!(
                        "unica.standards.{operation} failed while reading v8std MCP response"
                    ),
                    changes: Vec::new(),
                    warnings: Vec::new(),
                    errors: vec![err.to_string()],
                    artifacts: vec![endpoint, request.method.to_string()],
                    stdout: None,
                    stderr: None,
                    command: None,
                },
            },
            Err(err) => AdapterOutcome {
                ok: false,
                summary: format!(
                    "unica.standards.{operation} failed through internal v8std MCP proxy"
                ),
                changes: Vec::new(),
                warnings: Vec::new(),
                errors: vec![err.to_string()],
                artifacts: vec![endpoint, request.method.to_string()],
                stdout: None,
                stderr: None,
                command: None,
            },
        }
    }
}

fn select_params(args: &Map<String, Value>, keys: &[&str]) -> Value {
    let mut params = Map::new();
    for key in keys {
        if let Some(value) = args.get(*key) {
            params.insert((*key).to_string(), value.clone());
        }
    }
    Value::Object(params)
}

fn normalize_mcp_http_body(text: &str) -> String {
    let data_lines = text
        .lines()
        .filter_map(|line| line.strip_prefix("data:"))
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
    if data_lines.is_empty() {
        return text.to_string();
    }
    data_lines.join("\n")
}

fn cli_args(args: &Map<String, Value>) -> Vec<String> {
    if let Some(Value::Array(items)) = args.get("args") {
        return items.iter().map(value_to_cli_string).collect();
    }

    let mut result = Vec::new();
    for (key, value) in args {
        if matches!(key.as_str(), "dryRun" | "cwd" | "confirm") {
            continue;
        }
        let flag = format!("--{}", kebab_case(key));
        match value {
            Value::Bool(true) => result.push(flag),
            Value::Bool(false) | Value::Null => {}
            Value::Array(items) => {
                for item in items {
                    result.push(flag.clone());
                    result.push(value_to_cli_string(item));
                }
            }
            other => {
                result.push(flag);
                result.push(value_to_cli_string(other));
            }
        }
    }
    result
}

fn kebab_case(key: &str) -> String {
    let mut out = String::new();
    for (index, ch) in key.chars().enumerate() {
        if ch == '_' {
            out.push('-');
        } else if ch.is_ascii_uppercase() {
            if index > 0 {
                out.push('-');
            }
            out.push(ch.to_ascii_lowercase());
        } else {
            out.push(ch);
        }
    }
    out
}

#[allow(dead_code)]
fn _path_list(paths: &[PathBuf]) -> Vec<String> {
    paths
        .iter()
        .map(|path| path.display().to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn standards_search_maps_to_v8std_search_request() {
        let mut args = Map::new();
        args.insert("query".to_string(), json!("modal windows"));
        args.insert("limit".to_string(), json!(3));

        let request = StandardsAdapter::request_for("search", &args).unwrap();

        assert_eq!(request.method, "v8std_search");
        assert_eq!(request.params["query"], "modal windows");
        assert_eq!(request.params["limit"], 3);
    }

    #[test]
    fn standards_explain_prefers_diagnostics_codes() {
        let mut args = Map::new();
        args.insert("codes".to_string(), json!(["acc:142"]));
        args.insert("query".to_string(), json!("ignored when codes are present"));

        let request = StandardsAdapter::request_for("explain", &args).unwrap();

        assert_eq!(request.method, "v8std_explain_diagnostics");
        assert_eq!(request.params["codes"][0], "acc:142");
    }

    #[test]
    fn build_runtime_adapter_dry_run_builds_v8_runner_command() {
        let context = WorkspaceContext::discover(std::env::current_dir().unwrap()).unwrap();
        let mut args = Map::new();
        args.insert("sourceSet".to_string(), json!("main"));

        let outcome = CliAdapter::new("run-v8-runner.sh", &["build"], "build/runtime")
            .invoke("unica.build.load", &args, &context, true, true)
            .unwrap();

        let command = outcome.command.unwrap().join(" ");
        assert!(command.contains("run-v8-runner.sh"));
        assert!(command.contains("build"));
        assert!(command.contains("--source-set main"));
    }

    #[test]
    fn code_adapter_dry_run_builds_bsl_analyzer_command() {
        let context = WorkspaceContext::discover(std::env::current_dir().unwrap()).unwrap();
        let mut args = Map::new();
        args.insert("query".to_string(), json!("ОбщийМодуль"));

        let outcome = CliAdapter::new("run-bsl-analyzer.sh", &["search"], "code analysis")
            .invoke("unica.code.search", &args, &context, true, false)
            .unwrap();

        let command = outcome.command.unwrap().join(" ");
        assert!(command.contains("run-bsl-analyzer.sh"));
        assert!(command.contains("search"));
        assert!(command.contains("--query"));
        assert!(command.contains("ОбщийМодуль"));
    }
}
