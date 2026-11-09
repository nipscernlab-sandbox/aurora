#!/usr/bin/env python3
"""Monta o bundle de ferramentas da AURORA para uma plataforma.

O bundle é o único lugar de onde a AURORA executa ferramentas. Este script é o
único lugar que baixa ou compila alguma coisa: a AURORA, em uso, só lê o
bundle já montado.

    python3 scripts/bundle.py --out dist/toolchain
    python3 scripts/bundle.py --out dist/toolchain --only yanc
    python3 scripts/bundle.py --out dist/toolchain --platform windows-x64
    python3 scripts/bundle.py --surfer-prebuilt dist

Os pacotes, com as versões exatas, estão em bundle/versions.json:

- oss-cad-suite: release datada do OSS CAD Suite, baixada e conferida pelo
  SHA-256 publicado no GitHub. No Linux e no macOS, Icarus, Verilator,
  Yosys e dot; no Windows, só o Yosys;
- msys: só no Windows, o bloco MSYS2 UCRT64 que o repositório aurora-toolchain
  monta e testa (Icarus, Verilator, o g++, o make e o Perl que ele usa, e
  Python com cocotb). O zip e o manifesto da release, conferidos pelo
  SHA-256 fixado aqui;
- yanc: compilado do commit fixado (make stage);
- surfer-aurora: o executável pré-compilado do commit fixado do fork da
  AURORA, que o workflow surfer-aurora.yml publica numa pré-release deste
  repositório e versions.json fixa pelo SHA-256 (`prebuilt`, por
  plataforma). Sem ele para a plataforma, ou com AURORA_BUILD_SURFER,
  compilado do commit (cargo build --bin surfer, como o CI do fork), o que
  leva uns 14 minutos. --surfer-prebuilt DIR só compila e grava em DIR o
  pacote da plataforma, que o workflow publica;
- graphviz: só no Windows, onde o OSS CAD Suite não traz o dot; zip oficial
  conferido pelo SHA-256 publicado;
- studio: a AURORA IDE pronta, que o release.yml do repositório
  nipscernlab-sandbox/aurora-ide publica, uma por plataforma, e
  versions.json fixa pela URL e pelo SHA-256 (`prebuilt`). Com
  AURORA_IDE_SRC, compilada de um clone do aurora-ide (npm ci e tauri
  build), cuja versão tem que ser a de versions.json;
- aurora-learn: as trilhas de exercícios do `aurora learn`, copiadas de
  aurora-learn/ deste repositório. Só dados: quem as corrige é a AURORA.
- rtl-viewer: o modelo do HTML do RTL viewer (`aurora synth --open`),
  gerado de packages/rtl-viewer/ deste repositório (npm ci e npm run
  build). Só dados: quem o preenche com o netlist é a AURORA.

Os componentes que o instalador oferece estão em bundle/components.json. O
OSS CAD Suite vira quatro (icarus, verilator, yosys, graphviz): cada um leva
os arquivos que a ferramenta executa, os dados dela e o fecho das bibliotecas
dinâmicas que esses binários carregam (scripts/binaries.py). O resto do
pacote (nextpnr, GHDL, GTKWave, bases de FPGA...) fica de fora. Do msys,
cada componente leva os pacotes do MSYS2 que pede, com as dependências e os
arquivos que o manifesto da release lista para cada um.

Saída:

    <out>/bundle.json                 cabeçalho do manifesto
    <out>/components/<nome>.json      um por componente, com os hashes
    <out>/<dir do pacote>/...         os arquivos
    <out>.contents.json               que arquivo é de que componente, para
                                      os instaladores (não é instalado)

Só usa a biblioteca padrão do Python (3.9 ou mais novo). Compilar o YANC
exige gcc/clang, make, flex e bison; o surfer-aurora, cargo; o Studio (só com
AURORA_IDE_SRC), cargo, Node.js e, no Linux, as bibliotecas de
desenvolvimento do webkit2gtk 4.1. No Windows, rode dentro do shell MINGW64
do MSYS2.

Variáveis opcionais:
  AURORA_BUNDLE_CACHE   onde guardar downloads, clones e pacotes extraídos
                      (padrão: ./.bundle-cache)
  YANC_MAKE_ARGS      argumentos extras para o make do YANC
                      (no macOS: BISON=... FLEX=... do Homebrew)
  AURORA_MSYS_DIST      o dist/ de um build local do aurora-toolchain (com o
                      lace-msys-<tag>.zip e o .json), no lugar da release:
                      para testar um bloco de Windows antes de publicá-lo
  AURORA_BUILD_SURFER   compila o surfer-aurora mesmo com o pré-compilado
                      fixado em versions.json
  AURORA_IDE_SRC        um clone do aurora-ide (com o submódulo), de onde a
                      AURORA IDE é compilada no lugar da release
"""

import argparse
import fnmatch
import hashlib
import json
import os
import platform as pyplatform
import shlex
import shutil
import subprocess
import sys
import tarfile
import urllib.request
import zipfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import binaries  # noqa: E402

ROOT = Path(__file__).resolve().parent.parent
SCHEMA = 2
CONTENTS_SCHEMA = 1
PLATFORMS = ["linux-x64", "darwin-arm64", "windows-x64"]


def current_platform():
    system = pyplatform.system()
    machine = pyplatform.machine().lower()
    if system == "Linux" and machine in ("x86_64", "amd64"):
        return "linux-x64"
    if system == "Darwin" and machine in ("arm64", "aarch64"):
        return "darwin-arm64"
    if system == "Windows" or system.startswith(("MINGW", "MSYS")):
        return "windows-x64"
    sys.exit(f"plataforma sem bundle: {system} {machine}")


def log(msg):
    print(f"[bundle] {msg}", flush=True)


def sha256_file(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def download(url, sha256, cache):
    """Baixa para o cache (uma vez) e confere o SHA-256."""
    dest = cache / "downloads" / url.rsplit("/", 1)[-1]
    dest.parent.mkdir(parents=True, exist_ok=True)
    if not dest.is_file() or sha256_file(dest) != sha256:
        log(f"baixando {url}")
        tmp = dest.with_suffix(dest.suffix + ".part")
        with urllib.request.urlopen(url) as r, open(tmp, "wb") as f:
            shutil.copyfileobj(r, f, 1 << 20)
        tmp.replace(dest)
    got = sha256_file(dest)
    if got != sha256:
        sys.exit(f"SHA-256 não confere para {dest.name}: esperado {sha256}, veio {got}")
    return dest


def run(cmd, cwd=None, env=None):
    log("$ " + " ".join(shlex.quote(str(c)) for c in cmd))
    subprocess.run([str(c) for c in cmd], cwd=cwd, env=env, check=True)


def checkout(repo, commit, cache, name):
    """Clone raso do commit fixado."""
    dest = cache / "src" / name
    if (dest / ".git").is_dir():
        head = subprocess.run(["git", "-C", dest, "rev-parse", "HEAD"], capture_output=True, text=True).stdout.strip()
        if head == commit:
            return dest
        shutil.rmtree(dest)
    dest.mkdir(parents=True)
    run(["git", "init", "-q"], cwd=dest)
    run(["git", "remote", "add", "origin", repo], cwd=dest)
    run(["git", "fetch", "-q", "--depth", "1", "origin", commit], cwd=dest)
    run(["git", "checkout", "-q", "FETCH_HEAD"], cwd=dest)
    return dest


def extract_tgz(archive, out):
    with tarfile.open(archive) as tar:
        try:
            tar.extractall(out, filter="tar")
        except TypeError:  # Python < 3.12
            tar.extractall(out)


# ------------------------------------------------------------- pacotes
#
# Cada função deixa o pacote pronto num diretório e devolve (raiz, origem).


def pkg_oss_cad_suite(spec, plat, work, cache):
    asset = spec["assets"][plat]
    archive = download(asset["url"], asset["sha256"], cache)
    dest = cache / "extract" / archive.name.replace(".tgz", "")
    marker = dest / ".sha256"
    if not marker.is_file() or marker.read_text().strip() != asset["sha256"]:
        log(f"extraindo {archive.name}")
        shutil.rmtree(dest, ignore_errors=True)
        extract_tgz(archive, dest)
        marker.write_text(asset["sha256"])
    root = dest / "oss-cad-suite"
    if not root.is_dir():
        sys.exit("o pacote do OSS CAD Suite não trouxe oss-cad-suite/")
    return root, {"source": asset["url"], "sha256": asset["sha256"]}


def pkg_yanc(spec, plat, work, cache):
    src = checkout(spec["repository"], spec["commit"], cache, "yanc")
    stage = work / "yanc"
    shutil.rmtree(stage, ignore_errors=True)
    args = ["make", "-C", src, "stage", f"STAGE={stage}", "CFLAGS=-O2 -std=gnu17"]
    args += shlex.split(os.environ.get("YANC_MAKE_ARGS", ""))
    if plat == "windows-x64":
        args.append("OS=Windows_NT")
    run(args)
    return stage, {"source": f"git+{spec['repository']}@{spec['commit']}"}


def build_surfer(spec, plat, cache):
    """Compila o surfer-aurora do commit fixado; devolve o executável e o fonte."""
    src = checkout(spec["repository"], spec["commit"], cache, "surfer-aurora")
    target = cache / "surfer-target"
    # No Windows, o runtime do Visual C++ dentro do executável, como o da AURORA
    # (.cargo/config.toml): sem ele, o surfer-aurora pede o VCRUNTIME140.dll.
    env = dict(
        os.environ,
        CARGO_TARGET_DIR=str(target),
        CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_RUSTFLAGS="-C target-feature=+crt-static",
    )
    run(["cargo", "build", "--bin", "surfer", "--release", "--locked", "--features", "accesskit"], cwd=src, env=env)
    exe = ".exe" if plat == "windows-x64" else ""
    return target / "release" / f"surfer{exe}", src


def surfer_prebuilt(pins, plat, out, cache):
    """Compila o surfer-aurora e grava em out o pacote pré-compilado da
    plataforma: o executável e as licenças, num .tar.gz."""
    spec = pins["packages"]["surfer-aurora"]
    binary, src = build_surfer(spec, plat, cache)
    exe = ".exe" if plat == "windows-x64" else ""
    out.mkdir(parents=True, exist_ok=True)
    archive = out / f"surfer-aurora-{spec['version']}-{plat}.tar.gz"
    with tarfile.open(archive, "w:gz") as tar:
        tar.add(binary, arcname=f"surfer-aurora{exe}")
        for lic in sorted(src.glob("LICENSE*")):
            tar.add(lic, arcname=lic.name)
    log(f"{archive.name}: sha256 {sha256_file(archive)}")
    return archive


def pkg_surfer_aurora(spec, plat, work, cache):
    exe = ".exe" if plat == "windows-x64" else ""
    stage = work / "surfer-aurora"
    shutil.rmtree(stage, ignore_errors=True)
    stage.mkdir(parents=True)
    # O executável pré-compilado do mesmo commit (surfer-aurora.yml): sem
    # ele, cada release compilava o surfer-aurora de novo, uns 14 minutos
    # por plataforma.
    prebuilt = spec.get("prebuilt", {}).get(plat)
    if prebuilt and not os.environ.get("AURORA_BUILD_SURFER"):
        extract_tgz(download(prebuilt["url"], prebuilt["sha256"], cache), stage)
        if not (stage / f"surfer-aurora{exe}").is_file():
            sys.exit(f"o pacote pré-compilado do surfer-aurora não tem surfer-aurora{exe}")
    else:
        binary, src = build_surfer(spec, plat, cache)
        shutil.copy2(binary, stage / f"surfer-aurora{exe}")
        for lic in src.glob("LICENSE*"):
            shutil.copy2(lic, stage / lic.name)
    # O cliente web (WASM) da mesma tag, que a CI do fork publica: a AURORA
    # Studio o mostra numa aba, ligado a um `surfer-aurora server`. Um zip
    # só serve às três plataformas, e cliente e servidor precisam ser da
    # mesma versão.
    web = spec.get("web")
    if web:
        archive = download(web["url"], web["sha256"], cache)
        with zipfile.ZipFile(archive) as z:
            z.extractall(stage / "web")
        if not (stage / "web" / "index.html").is_file():
            sys.exit("o zip do cliente web do surfer-aurora não tem index.html na raiz")
    return stage, {"source": f"git+{spec['repository']}@{spec['commit']}"}


def pkg_studio(spec, plat, work, cache):
    """A AURORA IDE pronta, da release do aurora-ide (o `prebuilt` de
    versions.json, conferido pelo SHA-256): no macOS, o `AURORA IDE.app`; no
    Linux e no Windows, o executável, que usa o WebView do sistema
    (webkit2gtk 4.1, WebView2), e o ícone do atalho. Com AURORA_IDE_SRC
    apontando para um clone do aurora-ide, compila dele (build_studio)."""
    stage = work / "studio"
    shutil.rmtree(stage, ignore_errors=True)
    stage.mkdir(parents=True)
    exe = ".exe" if plat == "windows-x64" else ""
    main = "AURORA IDE.app" if plat == "darwin-arm64" else f"aurora-ide{exe}"
    local = os.environ.get("AURORA_IDE_SRC")
    if local:
        origin = build_studio(Path(local).resolve(), spec, plat, stage, cache)
    else:
        prebuilt = spec.get("prebuilt", {}).get(plat)
        if not prebuilt:
            sys.exit(
                f"studio: bundle/versions.json não tem a IDE pronta para {plat}; "
                "defina AURORA_IDE_SRC com um clone do aurora-ide para compilá-la"
            )
        extract_tgz(download(prebuilt["url"], prebuilt["sha256"], cache), stage)
        origin = {"source": prebuilt["url"], "sha256": prebuilt["sha256"]}
    if not (stage / main).exists():
        sys.exit(f"studio: o pacote da AURORA IDE {spec['version']} não tem {main}")
    return stage, origin


def build_studio(src, spec, plat, stage, cache):
    """Compila a AURORA IDE de um clone do aurora-ide, como o release.yml de
    lá (npm ci e tauri build), para testar a IDE e a AURORA juntas antes da
    release da IDE. A versão do clone tem que ser a de versions.json."""
    conf = json.loads((src / "src-tauri" / "tauri.conf.json").read_text(encoding="utf-8"))
    npm_package = json.loads((src / "package.json").read_text(encoding="utf-8"))
    for where, version in (("src-tauri/tauri.conf.json", conf["version"]), ("package.json", npm_package["version"])):
        if version != spec["version"]:
            sys.exit(f"studio: bundle/versions.json diz {spec['version']}, e {src / where} diz {version}")
    npm = shutil.which("npm")
    if not npm:
        sys.exit("studio: o build da AURORA IDE precisa do Node.js (npm)")
    if not (src / "node_modules").is_dir():
        run([npm, "ci", "--no-audit", "--no-fund"], cwd=src)
    target = cache / "studio-target"
    env = dict(os.environ, CARGO_TARGET_DIR=str(target))
    bundles = ["--bundles", "app"] if plat == "darwin-arm64" else ["--no-bundle"]
    run([npm, "exec", "--", "tauri", "build", "--ci", *bundles], cwd=src, env=env)
    if plat == "darwin-arm64":
        app = target / "release" / "bundle" / "macos" / f"{conf['productName']}.app"
        if not app.is_dir():
            sys.exit(f"studio: o tauri build não gerou {app}")
        shutil.copytree(app, stage / app.name, symlinks=True)
    else:
        exe = ".exe" if plat == "windows-x64" else ""
        shutil.copy2(target / "release" / f"aurora-ide{exe}", stage / f"aurora-ide{exe}")
        # O ícone que o instalador põe no atalho do menu de aplicativos.
        shutil.copy2(src / "src-tauri" / "icons" / "128x128.png", stage / "aurora-ide.png")
    return {"source": f"{src} (AURORA IDE {spec['version']}, compilada)"}


def pkg_aurora_learn(spec, plat, work, cache):
    """As trilhas do `aurora learn`, de aurora-learn/ deste repositório: textos,
    `.v` e `.json`, iguais nas três plataformas."""
    src = ROOT / spec["path"]
    if not any(src.glob("*/track.json")):
        sys.exit(f"aurora-learn: {src} não tem nenhuma trilha (<trilha>/track.json)")
    stage = work / "aurora-learn"
    shutil.rmtree(stage, ignore_errors=True)
    shutil.copytree(src, stage, ignore=shutil.ignore_patterns(".*"))
    return stage, {"source": f"{spec['path']}/ (AURORA {spec['version']})"}


def pkg_rtl_viewer(spec, plat, work, cache):
    """O modelo do HTML do RTL viewer, do build de packages/rtl-viewer/
    deste repositório: um arquivo só, igual nas três plataformas, com o
    viewer e o worker do layout embutidos."""
    src = ROOT / spec["path"]
    npm = shutil.which("npm")
    if not npm:
        sys.exit("rtl-viewer: o build do RTL viewer precisa do Node.js (npm)")
    if not (src / "node_modules").is_dir():
        run([npm, "ci", "--no-audit", "--no-fund"], cwd=src)
    run([npm, "run", "build"], cwd=src)
    template = src / "dist" / "rtl-viewer.html"
    if not template.is_file():
        sys.exit(f"rtl-viewer: o build não gerou {template}")
    stage = work / "rtl-viewer"
    shutil.rmtree(stage, ignore_errors=True)
    stage.mkdir(parents=True)
    shutil.copy2(template, stage / "rtl-viewer.html")
    return stage, {"source": f"{spec['path']}/ (AURORA {spec['version']})"}


def pkg_graphviz(spec, plat, work, cache):
    asset = spec["assets"][plat]
    archive = download(asset["url"], asset["sha256"], cache)
    stage = work / "graphviz"
    tmp = work / "graphviz-extract"
    shutil.rmtree(stage, ignore_errors=True)
    shutil.rmtree(tmp, ignore_errors=True)
    with zipfile.ZipFile(archive) as z:
        z.extractall(tmp)
    dots = list(tmp.rglob("bin/dot.exe"))
    if not dots:
        sys.exit("o zip do Graphviz não tem bin/dot.exe")
    shutil.move(str(dots[0].parent.parent), str(stage))
    shutil.rmtree(tmp, ignore_errors=True)
    return stage, {"source": asset["url"], "sha256": asset["sha256"]}


def pkg_msys(spec, plat, work, cache):
    """O bloco de Windows do aurora-toolchain: o zip (com msys/ na raiz) e o
    manifesto, que fica ao lado da pasta extraída (MSYS_MANIFEST)."""
    local = os.environ.get("AURORA_MSYS_DIST")
    if local:
        zips = sorted(Path(local).glob("lace-msys-*.zip"))
        if len(zips) != 1:
            sys.exit(f"AURORA_MSYS_DIST={local}: esperado um lace-msys-<tag>.zip, achados {len(zips)}")
        archive, manifest = zips[0], zips[0].with_suffix(".json")
        if not manifest.is_file():
            sys.exit(f"falta {manifest.name} ao lado de {archive.name}")
        sha256 = sha256_file(archive)
        log(f"msys de um build local: {archive}")
        origin = {"source": archive.resolve().as_uri(), "sha256": sha256}
    else:
        asset = spec["assets"][plat]
        if not asset.get("sha256") or not asset.get("manifest_sha256"):
            sys.exit(
                f"o pacote msys {spec['version']} ainda não tem SHA-256 em bundle/versions.json: "
                "publique a release no aurora-toolchain e preencha sha256 e manifest_sha256 "
                "(ou aponte AURORA_MSYS_DIST para o dist/ de um build local dele)"
            )
        archive = download(asset["url"], asset["sha256"], cache)
        manifest = download(asset["manifest"], asset["manifest_sha256"], cache)
        sha256 = asset["sha256"]
        origin = {"source": asset["url"], "sha256": sha256}

    dest = cache / "extract" / archive.stem
    marker = dest / ".sha256"
    if not marker.is_file() or marker.read_text().strip() != sha256:
        log(f"extraindo {archive.name}")
        shutil.rmtree(dest, ignore_errors=True)
        with zipfile.ZipFile(archive) as z:
            z.extractall(dest)
        marker.write_text(sha256)
    data = json.loads(Path(manifest).read_text(encoding="utf-8"))
    root = dest / data.get("root", "msys")
    if not root.is_dir():
        sys.exit(f"{archive.name} não trouxe {root.name}/")
    shutil.copy2(manifest, dest / MSYS_MANIFEST)
    return root, origin


# O manifesto da release do aurora-toolchain, guardado ao lado da pasta
# extraída: os pacotes do MSYS2, as dependências e os arquivos de cada um.
MSYS_MANIFEST = "lace-msys.json"
MSYS_PREFIX = "mingw-w64-ucrt-x86_64-"


PACKAGES = {
    "oss-cad-suite": pkg_oss_cad_suite,
    "msys": pkg_msys,
    "yanc": pkg_yanc,
    "surfer-aurora": pkg_surfer_aurora,
    "graphviz": pkg_graphviz,
    "studio": pkg_studio,
    "aurora-learn": pkg_aurora_learn,
    "rtl-viewer": pkg_rtl_viewer,
}


# ------------------------------------------------------------ seleção


def for_platform(mapping, plat, default=None):
    """O valor de `mapping` para `plat`: a chave da plataforma, 'unix' para
    Linux e macOS, ou '*'."""
    if not isinstance(mapping, dict):
        return mapping
    if plat in mapping:
        return mapping[plat]
    if plat != "windows-x64" and "unix" in mapping:
        return mapping["unix"]
    return mapping.get("*", default)


def matches(rel, pattern):
    if pattern == "**":
        return True
    if pattern.endswith("/**"):
        prefix = pattern[:-2]
        if not any(c in prefix for c in "*?["):
            return rel.startswith(prefix)
        # `pasta-*.egg/**`: o `*` do fnmatch também casa `/`.
        return fnmatch.fnmatchcase(rel, prefix + "*")
    return fnmatch.fnmatchcase(rel, pattern)


def walk(root):
    """Arquivos e symlinks do pacote, relativos à raiz, com '/'."""
    files = []
    for dirpath, dirnames, filenames in os.walk(root):
        base = Path(dirpath)
        # Symlink para diretório entra como arquivo, sem descer nele.
        for d in list(dirnames):
            if (base / d).is_symlink():
                dirnames.remove(d)
                filenames.append(d)
        for name in filenames:
            files.append((base / name).relative_to(root).as_posix())
    return sorted(files)


class Package:
    """Um pacote pronto, com os arquivos indexados para achar dependências."""

    def __init__(self, name, root, plat):
        self.name = name
        self.root = root
        self.plat = plat
        self.files = walk(root)
        self.present = set(self.files)
        # No Windows o carregador ignora maiúsculas.
        self.lower = {}
        for f in self.files:
            self.lower.setdefault(f.lower(), f)

    def symlink_closure(self, rel):
        """`rel` e, se for symlink, o destino dele dentro do pacote."""
        out = [rel]
        seen = {rel}
        while os.path.islink(self.root / rel):
            target = os.readlink(self.root / rel)
            if os.path.isabs(target):
                sys.exit(f"{self.name}/{rel} é symlink absoluto para {target}")
            rel = os.path.normpath(os.path.join(os.path.dirname(rel), target)).replace(os.sep, "/")
            if rel.startswith("..") or rel in seen:
                sys.exit(f"{self.name}/{out[0]}: symlink para fora do pacote ou em ciclo")
            seen.add(rel)
            out.append(rel)
        return out

    def resolve(self, binary, name, rpaths):
        """O arquivo do pacote que o carregador acharia para `name`, pedido
        por `binary`; None se não estiver no pacote. Devolve (caminho,
        é_do_sistema)."""
        here = os.path.dirname(binary)
        if self.plat == "linux-x64":
            # Os lançadores rodam o binário pelo ld-linux do pacote com
            # --library-path lib: quase tudo sai de lib/. O resto (as
            # bibliotecas do cocotb, que se acham entre si) vem do RUNPATH
            # com $ORIGIN, a pasta do binário.
            candidates = [f"lib/{name}"]
            for rp in rpaths:
                if "$ORIGIN" in rp:
                    base = rp.replace("${ORIGIN}", here).replace("$ORIGIN", here)
                    candidates.append(os.path.normpath(os.path.join(base, name)))
            for c in candidates:
                if c in self.present:
                    return c, False
            return None, False
        if self.plat == "darwin-arm64":
            if name.startswith(("/usr/lib/", "/System/")):
                return None, True
            # @executable_path é a pasta do programa que carregou a
            # biblioteca: os programas do pacote ficam em libexec/ (os
            # módulos .vpi do Icarus são carregados pelo vvp, de lá).
            exe_dirs = [here, "libexec", "bin"]
            candidates = []
            if name.startswith("@rpath/"):
                for rp in rpaths:
                    for d in exe_dirs:
                        base = rp.replace("@loader_path", here).replace("@executable_path", d)
                        candidates.append(os.path.join(base, name[len("@rpath/"):]))
            elif name.startswith("@loader_path/"):
                candidates.append(os.path.join(here, name[len("@loader_path/"):]))
            elif name.startswith("@executable_path/"):
                candidates += [os.path.join(d, name[len("@executable_path/"):]) for d in exe_dirs]
            for c in candidates:
                rel = os.path.normpath(c).replace(os.sep, "/")
                if rel in self.present:
                    return rel, False
            return None, False
        # Windows: a pasta do binário e o PATH que a AURORA monta (bin;lib).
        for d in (here, "bin", "lib"):
            rel = f"{d}/{name}".lower() if d else name.lower()
            if rel in self.lower:
                return self.lower[rel], False
        return None, True

    def closure(self, roots):
        """Os binários em `roots` e as bibliotecas que eles carregam, com os
        symlinks do caminho. Dependência que não é do sistema e não está no
        pacote é erro."""
        files, queue = set(), []
        for r in roots:
            for f in self.symlink_closure(r):
                if f not in files:
                    files.add(f)
                    queue.append(f)
        missing, system = [], set()
        while queue:
            rel = queue.pop()
            path = self.root / rel
            if path.is_symlink() or binaries.kind(path) is None:
                continue
            deps = binaries.dependencies(path)
            for name in deps.needed:
                found, is_system = self.resolve(rel, name, deps.rpaths)
                if found is None:
                    if is_system:
                        system.add(name)
                    else:
                        missing.append(f"{rel} -> {name}")
                    continue
                for f in self.symlink_closure(found):
                    if f not in files:
                        files.add(f)
                        queue.append(f)
        return files, missing, system


def select_msys(name, package, wanted):
    """Os arquivos dos pacotes do MSYS2 em `wanted` e das dependências deles,
    como o manifesto da release os lista."""
    manifest = json.loads((package.root.parent / MSYS_MANIFEST).read_text(encoding="utf-8"))
    by_name = {p["name"]: p for p in manifest["packages"]}

    def full(short):
        for candidate in (short, MSYS_PREFIX + short):
            if candidate in by_name:
                return candidate
        sys.exit(f"{name}: o manifesto do msys {manifest.get('tag')} não tem o pacote {short}")

    queue = [full(w) for w in wanted]
    taken = set()
    while queue:
        pkg = queue.pop()
        if pkg in taken:
            continue
        taken.add(pkg)
        queue.extend(d for d in by_name[pkg]["depends"] if d in by_name)
    files = set()
    for pkg in taken:
        files.update(by_name[pkg]["files"])
    missing = sorted(f for f in files if f not in package.present)
    if missing:
        sys.exit(f"{name}: o manifesto lista arquivos que o zip não tem (o primeiro: {missing[0]})")
    log(f"{name}: {len(taken)} pacotes do MSYS2")
    return files


def select(component, package, plat, common):
    """Os arquivos do pacote que o componente leva."""
    name = component["name"]
    sel = for_platform(component.get("select", {}), plat, [])
    data = for_platform(component.get("data", {}), plat, [])
    excl = for_platform(component.get("exclude", {}), plat, [])
    # Exceções ao exclude: o que entra mesmo casando com ele.
    keep_anyway = for_platform(component.get("keep", {}), plat, [])
    run_list = for_platform(component.get("run", {}), plat, [])
    msys = for_platform(component.get("msys", {}), plat, [])

    for pattern in sel + run_list:
        if not any(c in pattern for c in "*?[") and pattern not in package.present:
            sys.exit(f"{name}: o pacote {package.name} não tem {pattern}")

    def keep(rel, patterns):
        if not any(matches(rel, p) for p in patterns):
            return False
        return not any(matches(rel, p) for p in excl) or any(matches(rel, p) for p in keep_anyway)

    if msys:
        files = {f for f in select_msys(name, package, msys) if not any(matches(f, p) for p in excl)}
        return files, run_list

    roots = [f for f in package.files if keep(f, sel) or f in common]
    if component.get("closure", True):
        files, missing, system = package.closure(roots)
    else:
        files, missing, system = {g for f in roots for g in package.symlink_closure(f)}, [], set()
    if missing:
        sys.exit(f"{name}: dependências fora do pacote:\n  " + "\n  ".join(sorted(missing)))
    if system:
        log(f"{name}: bibliotecas do sistema: {', '.join(sorted(system, key=str.lower))}")
    for f in package.files:
        if keep(f, data):
            for g in package.symlink_closure(f):
                files.add(g)
    return files, run_list


def copy_file(src, dst):
    dst.parent.mkdir(parents=True, exist_ok=True)
    if dst.is_symlink() or dst.exists():
        return
    if src.is_symlink():
        os.symlink(os.readlink(src), dst)
    else:
        shutil.copy2(src, dst)


def size_of(path):
    return 0 if path.is_symlink() else path.stat().st_size


def check_profiles(catalog):
    """Confere os perfis de components.json: nomes únicos, componentes que
    existem e, em cada perfil, o que cada componente exige (requires)."""
    profiles = catalog.get("profiles", [])
    if not profiles:
        sys.exit("components.json: sem perfis (profiles)")
    known = {c["name"]: c for c in catalog["components"]}
    seen = set()
    for prof in profiles:
        name = prof.get("name", "")
        if not name or name in seen:
            sys.exit(f"components.json: perfil sem nome ou repetido: {name!r}")
        seen.add(name)
        for key in ("label", "description", "audience", "components"):
            if not prof.get(key):
                sys.exit(f"components.json: o perfil {name} não tem {key}")
        members = set(prof["components"])
        unknown = members - known.keys()
        if unknown:
            sys.exit(f"components.json: o perfil {name} tem componentes desconhecidos: {', '.join(sorted(unknown))}")
        for member in sorted(members):
            missing = set(known[member].get("requires", [])) - members
            if missing:
                sys.exit(f"components.json: no perfil {name}, {member} exige {', '.join(sorted(missing))}, que falta")


def check_translations(catalog):
    """Confere as traduções de components.json: cada idioma traduz a
    descrição de todos os componentes e o nome, a descrição e o público de
    todos os perfis, e nada que não existe."""
    components = {c["name"] for c in catalog["components"]}
    profiles = {p["name"] for p in catalog["profiles"]}
    for lang, texts in catalog.get("translations", {}).items():
        got = set(texts.get("components", {}))
        if got != components:
            sys.exit(
                f"components.json: a tradução {lang} dos componentes não bate: "
                f"faltam {sorted(components - got)}, sobram {sorted(got - components)}"
            )
        got = set(texts.get("profiles", {}))
        if got != profiles:
            sys.exit(
                f"components.json: a tradução {lang} dos perfis não bate: "
                f"faltam {sorted(profiles - got)}, sobram {sorted(got - profiles)}"
            )
        for name, prof in texts["profiles"].items():
            for key in ("label", "description", "audience"):
                if not prof.get(key):
                    sys.exit(f"components.json: a tradução {lang} do perfil {name} não tem {key}")


def component_translations(catalog, name, plat):
    """As traduções de um componente na plataforma: {idioma: {description}}."""
    return {
        lang: {"description": for_platform(texts["components"][name], plat)}
        for lang, texts in catalog.get("translations", {}).items()
    }


def platform_profiles(catalog, names):
    """Os perfis com só os componentes de `names` (os do bundle desta
    plataforma), na ordem do catálogo; um perfil que fica vazio sai."""
    out = []
    for prof in catalog["profiles"]:
        members = [n for n in prof["components"] if n in names]
        if members:
            out.append({
                "name": prof["name"],
                "label": prof["label"],
                "description": prof["description"],
                "audience": prof["audience"],
                "translations": {
                    lang: texts["profiles"][prof["name"]]
                    for lang, texts in catalog.get("translations", {}).items()
                },
                "components": members,
            })
    return out


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--out", type=Path, help="diretório do bundle (criado; precisa estar vazio)")
    parser.add_argument(
        "--surfer-prebuilt",
        type=Path,
        metavar="DIR",
        help="só compila o surfer-aurora e grava em DIR o pacote pré-compilado da plataforma",
    )
    parser.add_argument("--platform", default=None, choices=PLATFORMS)
    parser.add_argument("--only", help="componentes separados por vírgula (bundle parcial, para desenvolvimento)")
    parser.add_argument("--versions", type=Path, default=ROOT / "bundle" / "versions.json")
    parser.add_argument("--components", type=Path, default=ROOT / "bundle" / "components.json")
    args = parser.parse_args()
    plat = args.platform or current_platform()

    pins = json.loads(args.versions.read_text(encoding="utf-8"))
    cache = Path(os.environ.get("AURORA_BUNDLE_CACHE", ROOT / ".bundle-cache")).resolve()
    if args.surfer_prebuilt:
        surfer_prebuilt(pins, plat, args.surfer_prebuilt.resolve(), cache)
        return
    if args.out is None:
        parser.error("--out é obrigatório (ou --surfer-prebuilt)")
    # UTF-8 explícito: no Windows, o padrão do Python é a página do sistema
    # (cp1252), e os textos têm acentos.
    catalog = json.loads(args.components.read_text(encoding="utf-8"))
    check_profiles(catalog)
    check_translations(catalog)
    components = [c for c in catalog["components"] if for_platform(c["package"], plat) in pins["packages"]]
    components = [
        c for c in components
        if plat in pins["packages"][for_platform(c["package"], plat)].get("platforms", [plat])
    ]
    if args.only:
        only = [c.strip() for c in args.only.split(",")]
        known = {c["name"] for c in catalog["components"]}
        unknown = set(only) - known
        if unknown:
            sys.exit(f"componentes desconhecidos: {', '.join(sorted(unknown))}")
        components = [c for c in components if c["name"] in only]

    out = args.out.resolve()
    if out.exists() and any(out.iterdir()):
        sys.exit(f"{out} já existe e não está vazio")
    out.mkdir(parents=True, exist_ok=True)
    work = cache / "work" / plat
    work.mkdir(parents=True, exist_ok=True)

    packages = {}
    for pkg_name in dict.fromkeys(for_platform(c["package"], plat) for c in components):
        spec = pins["packages"][pkg_name]
        log(f"pacote {pkg_name} {spec['version']}")
        root, origin = PACKAGES[pkg_name](spec, plat, work, cache)
        packages[pkg_name] = (Package(pkg_name, root, plat), spec, origin)

    (out / "bundle.json").write_text(
        json.dumps({"schema": SCHEMA, "bundle": pins["bundle"], "platform": plat}, indent=2) + "\n"
    )
    (out / "components").mkdir()
    profiles = platform_profiles(catalog, {c["name"] for c in components})
    # O primeiro perfil é o padrão. O campo `recommended` dos componentes é
    # o pertencer a ele, para a AURORA anterior aos perfis, que lê o índice da
    # release nova no `aurora update` e exige o campo.
    default = set(profiles[0]["components"]) if profiles else set()
    contents = []
    for c in components:
        pkg_name = for_platform(c["package"], plat)
        package, spec, origin = packages[pkg_name]
        common = set(catalog.get("common", {}).get(pkg_name, []))
        common = {f for f in package.files if any(matches(f, p) for p in common)}
        files, run_list = select(c, package, plat, common)
        for rel in sorted(files):
            copy_file(package.root / rel, out / pkg_name / rel)
        manifest = {
            "name": c["name"],
            "version": spec["version"],
            "dir": pkg_name,
            **origin,
            "files": {f"{pkg_name}/{r}": sha256_file(out / pkg_name / r) for r in run_list},
        }
        manifest_rel = f"components/{c['name']}.json"
        (out / manifest_rel).write_text(
            json.dumps(manifest, indent=2, ensure_ascii=False) + "\n", encoding="utf-8"
        )
        paths = sorted(f"{pkg_name}/{r}" for r in files) + [manifest_rel]
        size = sum(size_of(out / p) for p in paths)
        log(f"{c['name']}: {len(paths)} arquivos, {size / 2**20:.0f} MiB")
        contents.append({
            "name": c["name"],
            "label": c["label"],
            "description": for_platform(c["description"], plat),
            "recommended": c["name"] in default,
            "requires": c.get("requires", []),
            "translations": component_translations(catalog, c["name"], plat),
            "version": spec["version"],
            "files": paths,
        })

    index = {
        "schema": CONTENTS_SCHEMA,
        "bundle": pins["bundle"],
        "platform": plat,
        "common": ["bundle.json"],
        "profiles": profiles,
        "components": contents,
    }
    contents_path = out.parent / f"{out.name}.contents.json"
    contents_path.write_text(
        json.dumps(index, indent=1, ensure_ascii=False) + "\n", encoding="utf-8"
    )
    total = sum(size_of(p) for p in out.rglob("*") if p.is_file() or p.is_symlink())
    log(f"bundle {pins['bundle']} para {plat} em {out}: {total / 2**20:.0f} MiB; índice em {contents_path}")


if __name__ == "__main__":
    main()
