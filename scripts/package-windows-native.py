#!/usr/bin/env python3
"""Package a native Windows build of Yappuccino as a portable ZIP and an Inno
Setup installer.

This is the Windows-native counterpart of scripts/package-windows.py, which
expects Linux, Wine and cargo xwin. It packages the executable produced by
scripts/build-windows on a real Windows machine, alongside the source
snapshot, dependency licenses and notices."""
import argparse
import json
import os
from pathlib import Path
import shutil
import struct
import subprocess
import tempfile

from release_package import ROOT, dependency_notices, digest, names_home, source_snapshot, zip_directory

TARGET = "x86_64-pc-windows-msvc"
# The subset of features scripts/build-windows builds for the Discord release.
FEATURES = ["--no-default-features", "--features", "desktop,discord"]
INNO_SETUP_VERSION = "7.1.0"
INNO_SETUP_DIR = ROOT / ".local/windows/inno-setup" / INNO_SETUP_VERSION / "InnoSetup"
WEBRTC_LICENSE = ROOT / ".local/windows/webrtc/LICENSE.md"


def check_executable(executable):
    with executable.open("rb") as stream:
        header = stream.read(64)
        if header[:2] != b"MZ":
            raise SystemExit(f"{executable} is not a Windows executable")
        stream.seek(struct.unpack_from("<I", header, 60)[0])
        pe = stream.read(6)
        if pe[:4] != b"PE\0\0" or struct.unpack_from("<H", pe, 4)[0] != 0x8664:
            raise SystemExit(f"{executable} is not an AMD64 Windows PE executable")
    contents = executable.read_bytes()
    if names_home(contents):
        print(f"warning: {executable} contains the build machine's home path")
    return contents


def compile_installer(package, version, output):
    """Compiles packaging/windows/yappuccino.iss over the package's files."""
    compiler = INNO_SETUP_DIR / "ISCC.exe"
    if not compiler.is_file():
        raise SystemExit(f"Inno Setup is not installed at {INNO_SETUP_DIR}")
    definitions = [f"/DAppVersion={version}", f"/DVersionNumber={version.split('-')[0]}",
                   f"/DSourceDir={package}", f"/DLicenseFile={ROOT / 'LICENSE'}", f"/O{output}"]
    subprocess.run([str(compiler), *definitions, str(ROOT / "packaging/windows/yappuccino.iss")],
                   check=True, timeout=1800)
    return output / f"Yappuccino-{version}-windows-x64-setup.exe"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--exe", type=Path, default=ROOT / "target" / "release" / "yappuccino.exe")
    parser.add_argument("--output", type=Path, default=ROOT / "dist")
    parser.add_argument("--no-installer", action="store_true", help="make only the portable ZIP")
    args = parser.parse_args()
    executable = args.exe.resolve()
    if not executable.is_file():
        raise SystemExit(f"{executable} does not exist; run the native build first")
    check_executable(executable)

    metadata = json.loads(subprocess.check_output(
        ["cargo", "metadata", "--locked", "--format-version", "1",
         "--filter-platform", TARGET, *FEATURES],
        cwd=ROOT, timeout=120,
    ))
    version = next(package["version"] for package in metadata["packages"] if package["name"] == "yappuccino")
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    base = f"Yappuccino-{version}"
    archive_path = output / f"{base}-windows-x64.zip"
    source_path = output / f"{base}-source.zip"

    skia_archives = sorted((ROOT / ".local/windows/skia/prepared").glob("*-static.tar.gz"))
    if len(skia_archives) != 1:
        raise SystemExit("Expected exactly one prepared Skia archive under .local/windows/skia/prepared")
    skia_archive = skia_archives[0]
    skia_source = ROOT / ".local/windows/skia/source/tree"
    if not skia_source.is_dir():
        raise SystemExit(f"Skia source is missing from {skia_source}")
    if not WEBRTC_LICENSE.is_file():
        raise SystemExit(f"libwebrtc's license is missing from {WEBRTC_LICENSE}")

    with tempfile.TemporaryDirectory(prefix="yappuccino-windows-package-") as temporary:
        temporary = Path(temporary)
        source_snapshot(metadata, base, temporary, source_path)

        package = output / f"{base}-windows-x64"
        if package.exists():
            shutil.rmtree(package)
        package.mkdir()
        shutil.copy2(executable, package / "Yappuccino.exe")
        shutil.copy2(ROOT / "LICENSE", package / "LICENSE.txt")
        (package / "SOURCE.txt").write_text(
            f"This folder holds Yappuccino {version}, a rebranded build of the\\n"
            "Yapfast nightly 0.4.0.\\n\\n"
            "The matching source tree is included beside this folder in the\\n"
            f"distribution, as {source_path.name}:\\n\\n"
            f"  {source_path.name}, SHA-256 in BUILD.json\\n\\n"
            "The upstream project it derives from is\\n"
            "https://github.com/Dev-NotAqua/yappuccino. The licenses folder holds\\n"
            "every dependency's own license and notices.\\n",
            encoding="utf-8", newline="\r\n",
        )
        (package / "README.txt").write_text(
            "Yappuccino for 64-bit Windows 10/11\n\n"
            "1. Extract the entire ZIP to a folder.\n"
            "2. Double-click Yappuccino.exe.\n"
            "3. Link your Discord account: Yappuccino shows a QR code, which Discord on\n"
            "   your phone scans (Settings, Scan QR Code). Discord doesn't allow apps\n"
            "   like Yappuccino on personal accounts; linking puts your account at some\n"
            "   risk of being suspended.\n"
            "4. Open a channel, or a voice channel to talk.\n\n"
            "No account or saved session is included.\n"
            "No Rust, Visual Studio, or separate app installation is required.\n"
            "Yappuccino writes diagnostics to the log in its data folder under\n"
            "%LOCALAPPDATA%, or to the console run-yappuccino.cmd opens.\n"
            "Windows stores session secrets in Credential Manager and app data\n"
            "under your Windows user profile. Keep the first verified session.\n\n"
            "Voice: select a voice channel and Join voice; the gear chooses devices.\n"
            "Use headphones for the first two-machine audio test.\n"
            "If anything fails, record the error text, Windows version, GPU and\n"
            "audio device model. Never send your password or Discord token in logs.\n\n"
            "If Yappuccino closes immediately, run run-yappuccino.cmd to keep its\n"
            "error visible.\n\n"
            "Yappuccino switches to software rendering if the GPU cannot start.\n"
            "To choose it yourself, run run-yappuccino.cmd --renderer software.\n"
            "To retry the GPU after a fallback, run run-yappuccino.cmd --renderer gpu.\n\n"
            "Yappuccino is free software under the GNU General Public License, version 3\n"
            "or later, in LICENSE.txt. Dependency notices and original licenses are\n"
            "included; SOURCE.txt says where this build's source code is.\n",
            encoding="utf-8",
        )
        (package / "run-yappuccino.cmd").write_text(
            '@echo off\ncd /d "%~dp0"\nYappuccino.exe %*\n'
            'if errorlevel 1 (\n  echo Yappuccino exited with an error.\n  pause\n)\n',
            encoding="utf-8", newline="\r\n",
        )

        dependencies = dependency_notices(metadata, package, package / "licenses", skia_archive,
                                          skia_source, WEBRTC_LICENSE.parent)
        (package / "THIRD-PARTY.json").write_text(json.dumps(dependencies, indent=2) + "\n", encoding="utf-8")

        build = {
            "version": version,
            "flavour": "discord",
            "target": TARGET,
            "built_on": "windows-native",
            "cargo_features": " ".join(FEATURES),
            "rustc": subprocess.check_output(["rustc", "--version"], text=True, timeout=10).strip(),
            "ui_framework": "Slint 1.19.0 (git ac7d423f)",
            "renderer_default": "skia-opengl, falling back to skia-software",
            "renderer_software": "skia-software",
            "skia_version": "0.157.0",
            "skia_source_revision": "6a8fe80eadfac737c12e64229a1b9530bc78f3d1",
            "skia_source_archive_sha256": "8401d0de878582a03abc1029bb62330aea58cda89904ae6ab187302e6d48ba5d",
            "skia_runtime": "MT_StaticRelease",
            "skia_archive_prepared_sha256": digest(skia_archive),
            "cargo_lock_sha256": digest(ROOT / "Cargo.lock"),
            "executable_sha256": digest(executable),
            "source_archive_sha256": digest(source_path),
            "webrtc_tag": "webrtc-51ef663",
            "webrtc_archive_sha256": "0a56a5c91b3b7b4222082b8a09388d16a802cde22d67295beb24c55700755b30",
        }
        (package / "BUILD.json").write_text(json.dumps(build, indent=2) + "\n", encoding="utf-8")

        zip_directory(package, archive_path)
        installer_path = None if args.no_installer else compile_installer(package, version, output)

    artifacts = [archive_path, source_path, *([installer_path] if installer_path else [])]
    (output / "SHA256SUMS.txt").write_text(
        "".join(f"{digest(path)}  {path.name}\n" for path in artifacts), encoding="utf-8")
    print(f"Packaged {archive_path} ({archive_path.stat().st_size / 1024 / 1024:.1f} MiB)")
    if installer_path:
        print(f"Installer: {installer_path} ({installer_path.stat().st_size / 1024 / 1024:.1f} MiB)")
    print(f"Source snapshot: {source_path}")
    print(f"App folder: {package}")
    print(f"Checksums: {output / 'SHA256SUMS.txt'}")


if __name__ == "__main__":
    main()
