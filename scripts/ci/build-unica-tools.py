#!/usr/bin/env python3
"""Build one target bundle of Unica tool binaries from third-party/tools.lock.json."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import platform
import shutil
import subprocess
import sys
import tarfile
import urllib.request
import zipfile
from pathlib import Path


def load_lock(path: Path) -> dict:
    lock = json.loads(path.read_text(encoding="utf-8"))
    if lock.get("schemaVersion") != 1:
        raise SystemExit(f"unsupported tools lock schemaVersion in {path}: {lock.get('schemaVersion')}")
    if not lock.get("targets") or not lock.get("tools"):
        raise SystemExit(f"invalid tools lock: {path}")
    return lock


def run(args: list[str], *, cwd: Path | None = None) -> None:
    print("+", " ".join(args), flush=True)
    subprocess.run(args, cwd=cwd, check=True)


def sha256(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            h.update(chunk)
    return h.hexdigest()


def download(url: str, dest: Path) -> None:
    dest.parent.mkdir(parents=True, exist_ok=True)
    print(f"download {url}", flush=True)
    with urllib.request.urlopen(url) as response, dest.open("wb") as out:
        shutil.copyfileobj(response, out)


def assert_host(target: str, targets: dict) -> None:
    cfg = targets[target]
    system = platform.system()
    machine = platform.machine().lower()
    supported_machines = {str(item).lower() for item in cfg["hostMachines"]}
    if system != cfg["hostSystem"] or machine not in supported_machines:
        expected = f"{cfg['hostSystem']} {sorted(supported_machines)}"
        actual = f"{system} {machine}"
        raise SystemExit(f"target {target} must be built on {expected}; current runner is {actual}")


def extract_v8_runner(archive: Path, binary_name: str, dest: Path) -> None:
    extract_dir = archive.parent / f"{archive.name}.extract"
    shutil.rmtree(extract_dir, ignore_errors=True)
    extract_dir.mkdir(parents=True)
    if archive.suffix == ".zip":
        with zipfile.ZipFile(archive) as zf:
            zf.extractall(extract_dir)
    else:
        with tarfile.open(archive) as tf:
            tf.extractall(extract_dir)

    matches = [p for p in extract_dir.rglob(binary_name) if p.is_file()]
    if not matches:
        raise SystemExit(f"{binary_name} not found in {archive}")
    shutil.copy2(matches[0], dest)


def verify_git_commit(source_dir: Path, expected_commit: str) -> None:
    if not expected_commit:
        return

    try:
        result = subprocess.run(
            ["git", "-C", str(source_dir), "rev-parse", "HEAD"],
            check=True,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
    except (FileNotFoundError, subprocess.CalledProcessError):
        return

    actual = result.stdout.strip()
    if actual != expected_commit:
        raise SystemExit(f"{source_dir} is at {actual}, expected {expected_commit}")


def checkout_source(tool: dict, work_dir: Path) -> Path:
    source_dir = work_dir / "source" / tool["name"]
    shutil.rmtree(source_dir, ignore_errors=True)
    source_dir.parent.mkdir(parents=True, exist_ok=True)
    run(
        [
            "git",
            "clone",
            "--depth",
            "1",
            "--branch",
            tool["sourceTag"],
            tool["repository"],
            str(source_dir),
        ]
    )
    verify_git_commit(source_dir, tool["sourceCommit"])
    return source_dir


def resolve_source(tool: dict, explicit_source: Path | None, work_dir: Path) -> Path:
    if explicit_source is not None:
        if not explicit_source.exists():
            raise SystemExit(f"{tool['name']} source directory not found: {explicit_source}")
        verify_git_commit(explicit_source, tool["sourceCommit"])
        return explicit_source

    return checkout_source(tool, work_dir)


def create_python_env(source_dir: Path, work_dir: Path) -> Path:
    if not source_dir.exists():
        raise SystemExit(f"python tool source directory not found: {source_dir}")

    venv_dir = (work_dir / "python-env").resolve()
    if os.name == "nt":
        venv_python = venv_dir / "Scripts" / "python.exe"
    else:
        venv_python = venv_dir / "bin" / "python"

    if not venv_python.exists():
        run([sys.executable, "-m", "venv", str(venv_dir)])

    run([str(venv_python), "-m", "pip", "install", "--upgrade", "pip", "pyinstaller"])
    run([str(venv_python), "-m", "pip", "install", str(source_dir)])
    return venv_python


def resolve_console_script_entrypoint(venv_python: Path, command_name: str) -> tuple[str, str]:
    code = r"""
import json
import sys
from importlib.metadata import entry_points

command_name = sys.argv[1]
eps = entry_points()
if hasattr(eps, "select"):
    candidates = eps.select(group="console_scripts", name=command_name)
else:
    candidates = [ep for ep in eps.get("console_scripts", []) if ep.name == command_name]

matches = list(candidates)
if not matches:
    raise SystemExit(f"console_scripts entrypoint not found: {command_name}")

entrypoint = matches[0]
if not entrypoint.attr:
    raise SystemExit(f"console_scripts entrypoint is not callable: {entrypoint.value}")

print(json.dumps({"module": entrypoint.module, "attr": entrypoint.attr}))
"""
    try:
        result = subprocess.run(
            [str(venv_python), "-c", code, command_name],
            check=True,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
    except subprocess.CalledProcessError as exc:
        detail = (exc.stderr or exc.stdout).strip()
        message = f": {detail}" if detail else ""
        raise SystemExit(f"failed to resolve installed entrypoint {command_name}{message}") from exc

    data = json.loads(result.stdout)
    return data["module"], data["attr"]


def write_entrypoint_stub(build_root: Path, command_name: str, module: str, attr: str) -> Path:
    stub = build_root / f"{command_name}-entrypoint.py"
    stub.write_text(
        "\n".join(
            [
                "import importlib",
                "import sys",
                "",
                f"MODULE = {module!r}",
                f"CALLABLE = {attr!r}",
                "",
                "",
                "def _load_entrypoint():",
                "    obj = importlib.import_module(MODULE)",
                "    for part in CALLABLE.split('.'):",
                "        obj = getattr(obj, part)",
                "    return obj",
                "",
                "",
                "if __name__ == '__main__':",
                "    sys.exit(_load_entrypoint()())",
                "",
            ]
        ),
        encoding="utf-8",
    )
    return stub


def build_python_entrypoint(
    tool: dict,
    work_dir: Path,
    out_dir: Path,
    exe: str,
    venv_python: Path,
) -> Path:
    command_name = tool["entrypoint"]
    build_root = (work_dir / command_name).resolve()
    shutil.rmtree(build_root, ignore_errors=True)
    build_root.mkdir(parents=True)

    module, attr = resolve_console_script_entrypoint(venv_python, command_name)
    script = write_entrypoint_stub(build_root, command_name, module, attr)
    collect_package = tool.get("collectAll", module.split(".", 1)[0])
    run(
        [
            str(venv_python),
            "-m",
            "PyInstaller",
            "--onefile",
            "--clean",
            "--noconfirm",
            "--name",
            command_name,
            "--collect-all",
            collect_package,
            "--hidden-import",
            module,
            str(script),
        ],
        cwd=build_root,
    )
    produced = build_root / "dist" / f"{command_name}{exe}"
    if not produced.exists():
        raise SystemExit(f"PyInstaller output not found: {produced}")

    dest = out_dir / f"{tool['binaryName']}{exe}"
    shutil.copy2(produced, dest)
    return dest


def tool_entry(
    *,
    target: str,
    target_triple: str,
    name: str,
    version: str,
    repository: str,
    tag: str,
    commit: str,
    license_id: str,
    binary: Path,
    relative_binary: str,
) -> dict:
    return {
        "name": name,
        "version": version,
        "repository": repository,
        "upstreamUrl": f"{repository}/releases/tag/{tag}",
        "sourceTag": tag,
        "sourceCommit": commit,
        "license": license_id,
        "target": target,
        "targetTriple": target_triple,
        "binaryPath": relative_binary,
        "sha256": sha256(binary),
    }


def main() -> None:
    if sys.version_info < (3, 10):
        raise SystemExit("build-unica-tools.py requires Python >= 3.10 because rlm-tools-bsl requires >= 3.10")

    parser = argparse.ArgumentParser()
    parser.add_argument("--target", required=True)
    parser.add_argument("--lock-file", type=Path, default=Path("plugins/unica/third-party/tools.lock.json"))
    parser.add_argument("--rlm-source", type=Path)
    parser.add_argument("--out-dir", type=Path, required=True)
    parser.add_argument("--work-dir", type=Path, default=Path(".build/unica-tools"))
    args = parser.parse_args()

    lock = load_lock(args.lock_file)
    targets = lock["targets"]
    if args.target not in targets:
        raise SystemExit(f"unknown target {args.target}; expected one of {', '.join(sorted(targets))}")

    assert_host(args.target, targets)
    cfg = targets[args.target]
    exe = cfg["exe"]

    target_bin_dir = args.out_dir / "bin" / args.target
    downloads_dir = args.work_dir / args.target / "downloads"
    target_bin_dir.mkdir(parents=True, exist_ok=True)
    downloads_dir.mkdir(parents=True, exist_ok=True)

    built_paths: dict[str, Path] = {}
    python_env_cache: dict[Path, Path] = {}
    source_cache: dict[tuple[str, str, str], Path] = {}

    for tool in lock["tools"]:
        asset = tool["assets"].get(args.target)
        if not asset:
            raise SystemExit(f"{tool['name']} has no asset for target {args.target}")

        strategy = tool["assetStrategy"]
        dest = target_bin_dir / f"{tool['binaryName']}{exe}"

        if strategy == "direct-release-asset":
            url = f"{tool['repository']}/releases/download/{tool['sourceTag']}/{asset['assetName']}"
            downloaded = downloads_dir / asset["assetName"]
            download(url, downloaded)
            shutil.copy2(downloaded, dest)
        elif strategy == "archive-release-asset":
            url = f"{tool['repository']}/releases/download/{tool['sourceTag']}/{asset['assetName']}"
            downloaded = downloads_dir / asset["assetName"]
            download(url, downloaded)
            extract_v8_runner(downloaded, asset["archiveBinary"], dest)
        elif strategy == "pyinstaller-entrypoint":
            key = (tool["repository"], tool["sourceTag"], tool["sourceCommit"])
            if key not in source_cache:
                source_cache[key] = resolve_source(tool, args.rlm_source, args.work_dir / args.target)
            source_dir = source_cache[key]
            if source_dir not in python_env_cache:
                python_env_cache[source_dir] = create_python_env(source_dir, args.work_dir / args.target)
            venv_python = python_env_cache[source_dir]
            dest = build_python_entrypoint(
                tool,
                args.work_dir / args.target / "pyinstaller",
                target_bin_dir,
                exe,
                venv_python,
            )
        else:
            raise SystemExit(f"unsupported assetStrategy for {tool['name']}: {strategy}")

        built_paths[tool["name"]] = dest

    for path in target_bin_dir.iterdir():
        if path.is_file() and not path.name.endswith(".exe"):
            path.chmod(path.stat().st_mode | 0o755)

    tools = [
        tool_entry(
            target=args.target,
            target_triple=cfg["targetTriple"],
            name=tool["name"],
            version=tool["version"],
            repository=tool["repository"],
            tag=tool["sourceTag"],
            commit=tool["sourceCommit"],
            license_id=tool["license"],
            binary=built_paths[tool["name"]],
            relative_binary=f"bin/{args.target}/{built_paths[tool['name']].name}",
        )
        for tool in lock["tools"]
    ]

    (args.out_dir / "tools.json").write_text(
        json.dumps(
            {
                "target": args.target,
                "targetTriple": cfg["targetTriple"],
                "lockFile": str(args.lock_file),
                "tools": tools,
            },
            ensure_ascii=False,
            indent=2,
        )
        + "\n",
        encoding="utf-8",
    )


if __name__ == "__main__":
    main()
