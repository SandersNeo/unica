from __future__ import annotations

import importlib.util
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch


def load_build_module():
    module_path = Path(__file__).resolve().parents[2] / "scripts" / "ci" / "build-unica-tools.py"
    spec = importlib.util.spec_from_file_location("build_unica_tools", module_path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"failed to load {module_path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class BuildPythonEntrypointTests(unittest.TestCase):
    def test_pyinstaller_uses_generated_python_stub_for_entrypoint(self) -> None:
        module = load_build_module()

        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            work_dir = root / "pyinstaller"
            out_dir = root / "out"
            out_dir.mkdir()
            calls = []

            def fake_run(args, *, cwd=None):
                calls.append((args, cwd))
                dist = Path(cwd) / "dist"
                dist.mkdir()
                (dist / "rlm-tools-bsl.exe").write_bytes(b"frozen")

            with (
                patch.object(
                    module,
                    "resolve_console_script_entrypoint",
                    return_value=("rlm_tools_bsl.server", "main"),
                ),
                patch.object(module, "run", side_effect=fake_run),
            ):
                dest = module.build_python_entrypoint(
                    {
                        "entrypoint": "rlm-tools-bsl",
                        "binaryName": "rlm-tools-bsl",
                    },
                    work_dir,
                    out_dir,
                    ".exe",
                    root / "venv" / "Scripts" / "python.exe",
                )

            stub = work_dir / "rlm-tools-bsl" / "rlm-tools-bsl-entrypoint.py"
            self.assertEqual(dest, out_dir / "rlm-tools-bsl.exe")
            self.assertEqual(dest.read_bytes(), b"frozen")
            self.assertTrue(stub.exists())
            self.assertIn("MODULE = 'rlm_tools_bsl.server'", stub.read_text(encoding="utf-8"))

            args, cwd = calls[0]
            self.assertEqual(cwd, (work_dir / "rlm-tools-bsl").resolve())
            self.assertEqual(Path(args[-1]).resolve(), stub.resolve())
            self.assertEqual(args[args.index("--collect-all") + 1], "rlm_tools_bsl")
            self.assertEqual(args[args.index("--hidden-import") + 1], "rlm_tools_bsl.server")
            self.assertFalse(str(args[-1]).endswith(".exe"))


if __name__ == "__main__":
    unittest.main()
