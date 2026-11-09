# O bundle de ferramentas

A AURORA só executa ferramentas de um bundle instalado junto com ele. Nunca do
`PATH`, nunca de um caminho configurado pelo usuário. As exceções, decididas
pelo autor, são o compilador C++ do Verilator no Linux e no macOS (no
Windows ele vem no bundle) e o Quartus Prime, para as placas Intel; e o
`taskkill` encerra um passo cancelado quando o sistema recusa o Job Object
dele (seção 4).

As duas famílias de plataforma montam o bundle de fontes diferentes:

- **Linux e macOS** acompanham o OSS CAD Suite: Icarus, Verilator, cocotb,
  Yosys, `dot` e openFPGALoader saem da mesma release datada dele, e o
  compilador, o `make` e o Perl do Verilator vêm do sistema.
- **Windows** tira Icarus e Verilator do bloco MSYS2 UCRT64 do repositório
  [aurora-toolchain](https://github.com/nipscernlab-sandbox/aurora-toolchain), que traz
  também o g++, o `make` e o Perl que o Verilator usa, e o Python com o
  cocotb. O Yosys e o openFPGALoader continuam vindo do OSS CAD Suite, e o
  `dot` do Graphviz.

O motivo: no Windows o OSS CAD Suite não traz compilador, `make`, Perl nem
cocotb, e o cocotb precisa de simulador, compilador e Python que casem.

## 1. O que vem no bundle

Bundle `2026.10.22`. Os pacotes, com as versões fixadas em
`bundle/versions.json`:

| Pacote | Versão | O que traz | De onde vem |
|---|---|---|---|
| YANC | v9.1 (`bf3d0da`) | `cmmcomp`, `asmcomp`, `cppcomp`, a biblioteca SAPHO, macros e headers (até a 6.x vinham também o `appcomp` e o `cpppp`, que a 7.1 juntou ao `asmcomp` e ao `cppcomp`) | compilado do fonte pelo empacotamento |
| surfer-aurora | v0.7.0-nips.10 (`d0af8a7`) | o fork do Surfer da AURORA e o cliente web (WASM) dele | o executável, pré-compilado do commit fixado pelo workflow `surfer-aurora.yml` numa pré-release deste repositório e conferido pelo SHA-256 fixado (`prebuilt`); sem ele, compilado do fonte pelo empacotamento; o cliente web, o zip que a CI do fork publica na mesma tag, conferido pelo SHA-256 fixado |
| OSS CAD Suite | release 2026-10-22 | Linux e macOS: Icarus Verilog, Verilator, o cocotb com o Python que o roda, Yosys, o `dot` do Graphviz e o openFPGALoader. Windows: o Yosys e o openFPGALoader | pacote oficial, conferido pelo SHA-256 publicado |
| msys | `ucrt64-v2` | Icarus Verilog, Verilator, o g++, o `make` e o Perl que ele usa, Python com cocotb | só no Windows: release do aurora-toolchain, o zip e o manifesto conferidos pelo SHA-256 fixado |
| Graphviz | 16.1.0 | `dot` | só no Windows (o OSS CAD Suite de Windows não traz), zip oficial conferido pelo SHA-256 publicado |
| studio | 8.1.0, a da AURORA IDE | a AURORA IDE: o executável `aurora-ide` (Linux, Windows) ou o `AURORA IDE.app` (macOS) | pronta, da release do [aurora-ide](https://github.com/nipscernlab-sandbox/aurora-ide), um arquivo por plataforma conferido pelo SHA-256 fixado (`prebuilt`); com `AURORA_IDE_SRC`, compilada de um clone do aurora-ide (`npm ci`, `tauri build`) |
| aurora-learn | a versão da AURORA | as trilhas de exercícios do `aurora learn` ([LEARN.md](LEARN.md)): textos, `.v` e `.json` | copiadas de `aurora-learn/` deste repositório |
| rtl-viewer | a versão da AURORA | o modelo do HTML do RTL viewer (`rtl-viewer.html`), que o `aurora synth --open` preenche com o netlist: o viewer, o React e o worker do layout num arquivo só | gerado de `packages/rtl-viewer/` deste repositório pelo empacotamento (`npm ci`, `npm run build`) |

Os componentes, que o instalador deixa escolher, estão em
`bundle/components.json`, junto dos perfis de instalação (`profiles`), que
juntam componentes por tipo de trabalho
([INSTALL.md, Idioma, perfis e componentes](INSTALL.md#idioma-perfis-e-componentes)):

| Componente | Pacote | Diretório |
|---|---|---|
| `yanc` | YANC | `yanc/` |
| `icarus` | OSS CAD Suite (Linux, macOS), msys (Windows) | `oss-cad-suite/` ou `msys/` |
| `verilator` | OSS CAD Suite (Linux, macOS), msys (Windows) | `oss-cad-suite/` ou `msys/` |
| `cocotb` | OSS CAD Suite (Linux, macOS), msys (Windows) | `oss-cad-suite/` ou `msys/` |
| `yosys` | OSS CAD Suite | `oss-cad-suite/` |
| `graphviz` | OSS CAD Suite (Linux, macOS), Graphviz (Windows) | `oss-cad-suite/` ou `graphviz/` |
| `surfer-aurora` | surfer-aurora | `surfer-aurora/` |
| `openfpgaloader` | OSS CAD Suite | `oss-cad-suite/` |
| `studio` | studio | `studio/` |
| `aurora-learn` | aurora-learn | `aurora-learn/` |
| `rtl-viewer` | rtl-viewer | `rtl-viewer/` |

A AURORA não roda o Studio: ele está no bundle para o instalador e o
`aurora install` o oferecerem como os outros, e o Studio instalado usa o
bundle em que está.
Liga só às bibliotecas do sistema (`closure: false`): no Linux, ao
webkit2gtk 4.1 e ao GTK 3; no Windows, ao WebView2.

O componente `openfpgaloader` grava as placas FPGA pelo cabo USB delas
([FPGA.md](FPGA.md)): o `openFPGALoader` com a libusb, a libftdi e o hidapi
do pacote, nas três plataformas (13 MiB no Linux, 5 MiB no macOS; no
Windows, a maior parte é a `libstdc++` que ele divide com o Yosys). Os
bitstreams auxiliares de gravar flash (`share/openFPGALoader`) ficam de
fora: nenhum serve às placas que a AURORA conhece. O pacote não traz a regra
do udev do openFPGALoader, que o Linux exige para acessar o cabo sem root.

O OSS CAD Suite inteiro tem 2,5 GB no Linux: traz também nextpnr, GHDL,
GTKWave, bases de FPGA e um Python com dezenas de pacotes. O nextpnr dele
cobre Lattice, Gowin e GateMate; o `nextpnr-mistral`, do Cyclone V, não vem
na release 2026-10-22 em nenhuma plataforma, embora a regra de build exista
no repositório do OSS CAD Suite. O empacotamento
leva de cada ferramenta só o que ela executa (os lançadores de `bin/`, os
binários de `libexec/`), os dados dela (`share/yosys`, `lib/ivl`,
`share/verilator`, a biblioteca padrão do Python para o `make` do Verilator,
as fontes do `dot`) e o fecho das bibliotecas dinâmicas que esses binários
carregam, calculado lendo os binários (ELF, Mach-O e PE, em
`scripts/binaries.py`). Dependência que não está no pacote e não é do sistema
(`/usr/lib` e `/System` no macOS, DLLs do Windows) faz o empacotamento
falhar. O bundle completo fica com 300 MiB no Linux; a divisão do macOS dá
141 MiB, sem o surfer-aurora (medidos na montagem local; o CI monta os
completos). O cocotb não entra nessas contas: acrescenta 42 MiB no Linux e
25 MiB no macOS a quem tem o Verilator, que já traz o mesmo Python, e 115 e
76 MiB a quem não tem. No Windows, o bloco MSYS2 (`ucrt64-v2`) se divide em
12 MiB para o Icarus, 630 MiB para o Verilator (quase tudo o g++ e o Perl)
e 100 MiB para o cocotb (o Python, a libstdc++ e o cocotb).

O cocotb leva o Python do pacote com a biblioteca padrão, o egg do cocotb
(as bibliotecas dele e a VPI de cada simulador), o `find_libpython` e o
pytest com o que ele importa (`pluggy`, `iniconfig`, `packaging`,
`pygments`). Do `site-packages`, só isso: os pacotes que ficam são as
exceções de `keep` em `bundle/components.json`.

O bloco MSYS2 se divide de outro jeito: o manifesto da release do
aurora-toolchain (`lace-msys-<tag>.json`) lista os pacotes do MSYS2 que estão
no zip, as dependências e os arquivos de cada um. Cada componente pede os
pacotes de que precisa (`msys` em `bundle/components.json`), e o
empacotamento leva esses pacotes, as dependências deles e os arquivos que o
manifesto lista. O Icarus leva o pacote `iverilog`; o Verilator,
`verilator`, `perl`, `gcc`, `python` e a camada MSYS que o `make` usa
(`make`, `bash`, `coreutils`, `findutils`, `msys2-runtime`).

Fica de fora de propósito: os plugins do Yosys (`share/yosys/plugins`, que
puxariam o GHDL), o `site-packages` (menos o que o cocotb usa) e os testes
do Python, e as cópias de depuração do Verilator.

O OSS CAD Suite compila cada ferramenta do `master` e publica uma release por
dia. Fixar a data e o hash do pacote é o que dá a versão exata. O bloco
MSYS2 tem versões estáveis, travadas pelo lock do aurora-toolchain: no
`ucrt64-v2`, Icarus 13.0, Verilator 5.050, gcc 16.2, Perl 5.44, Python 3.14
e cocotb 2.1.0. Na release 2026-10-22 do OSS CAD Suite as ferramentas se
identificam assim:

| Ferramenta | Versão reportada (Linux x64) |
|---|---|
| Yosys | 0.70+190 (git 485b63564) |
| Icarus Verilog | 14.0 (devel) s20260301-638-ge072b3334 |
| Verilator | 5.053 devel rev v5.052-528-gfd500fbb1 |
| cocotb | 2.1.0.dev0+41564633, no Python 3.11.6 do pacote |
| dot | Graphviz 2.43 (20190912) |
| openFPGALoader | v1.2.0 |

## 2. Plataformas

| Plataforma | Nome no bundle | Observação |
|---|---|---|
| Linux x64 | `linux-x64` | |
| macOS Apple Silicon | `darwin-arm64` | O OSS CAD Suite avisou que vai parar de publicar o macOS Intel, que a AURORA não suporta. |
| Windows 10 e 11 x64 | `windows-x64` | |

Um bundle só abre na plataforma para a qual foi montado.

## 3. Onde fica e como a AURORA o usa

```
<instalação>/
  bin/aurora[.exe]
  toolchain/
    bundle.json               formato, versão do bundle, plataforma
    components/<nome>.json    um por componente instalado
    yanc/                     bin/ SAPHO/ Macros/ Header/
    oss-cad-suite/            a parte do OSS CAD Suite dos componentes instalados
    msys/                     só no Windows: ucrt64/ e usr/bin/ do bloco MSYS2
    surfer-aurora/            surfer-aurora[.exe], web/ (o cliente web: index.html, surfer.js, surfer_bg.wasm)
    graphviz/                 só no Windows
    studio/                   a AURORA IDE: aurora-ide[.exe], ou AURORA IDE.app no macOS
    aurora-learn/               as trilhas do aurora learn
    rtl-viewer/               rtl-viewer.html, o modelo do HTML do aurora synth --open
```

A AURORA procura `toolchain/` ao lado de `bin/` (ou ao lado do executável,
com os symlinks resolvidos).
`aurora --toolchain <DIR>` usa outro bundle, para desenvolvimento; ele também
precisa ter `bundle.json`.

Cada `components/<nome>.json` tem a versão, a origem e o SHA-256 de cada
executável do componente que a AURORA roda. `aurora tools --verify` confere
esses hashes. Um componente sem arquivo em `components/` não está instalado.

Como cada ferramenta é executada:

| Ferramenta | Linux e macOS | Windows |
|---|---|---|
| YANC | o binário em `yanc/bin/`; da 7.1 em diante, cada um acha `SAPHO/`, `Macros/` e `Header/` em `../` a partir do próprio executável, então as quatro pastas andam juntas | `yanc/bin/*.exe`, idem |
| Icarus | `/bin/bash oss-cad-suite/bin/<ferramenta>`: o lançador do pacote, que carrega o binário de `libexec/` com as bibliotecas do próprio pacote | o `.exe` de `msys/ucrt64/bin/`, com `PATH` nesse diretório, onde estão as DLLs |
| Yosys, `dot` | como o Icarus | o `.exe` de `oss-cad-suite/bin/` (ou `graphviz/bin/dot.exe`) com `PATH` em `bin;lib` do pacote, como o `environment.bat` |
| Verilator | Perl do sistema com o script `oss-cad-suite/bin/verilator`, que roda o `bin/verilator_bin` do pacote; o `make` e o compilador são os do sistema; o `make` gerado usa o Python do pacote (`PYTHON3=`) | o `msys/ucrt64/bin/verilator_bin.exe`, direto, sem o script Perl `msys/ucrt64/bin/verilator`, com o `VERILATOR_ROOT` em `msys/ucrt64/share/verilator` (pelo script, um caminho com espaço ou acento chegava trocado; ver [API.md, seção 5.2](API.md#52-simulatetoolchain-processor-simulationoptions-control---resultsimulationresult)); o `make` é o da camada MSYS (`msys/usr/bin/make.exe`), que roda o `verilated.mk` com o `sh` dela e compila com o `g++` de `ucrt64/bin`; o `make` gerado usa `msys/ucrt64/bin/python.exe` |
| surfer-aurora | o binário, com as variáveis de display | o `.exe` |

Todo processo parte de um ambiente vazio. Os lançadores recebem
`PATH=/usr/bin:/bin` para o `bash`, o `dirname` e o `readlink` que usam. No
Windows, o `PATH` das ferramentas do pacote termina em
`%SystemRoot%\System32`, e o `ComSpec` é repassado: o `iverilog` roda o
`ivlpp` e o `ivl` pelo `system()` da biblioteca C, que usa o `cmd.exe`.

A AURORA roda o cocotb com o Icarus e com o Verilator ([API.md, seção
5.3.2](API.md#532-testbench-cocotb)): o Python do componente
(`oss-cad-suite/bin/tabbypy3` no Linux e no macOS,
`msys/ucrt64/bin/python.exe` no Windows) roda uma sonda que diz à AURORA
onde estão as VPIs, o `verilator.cpp` do cocotb e a biblioteca do Python.
O `vvp` carrega a VPI com o ambiente que ela descreve; no Verilator, a VPI
entra na ligação do modelo, que roda com o mesmo ambiente. No Windows o
cocotb do MSYS2 traz a VPI do Verilator estática (`libcocotbvpi_verilator.a`).
O cocotb no Verilator foi testado no Windows; no Linux e no macOS, não.
Quem rodar à mão com o Verilator tem de passar ao `make` o Python do pacote
(`PYTHON3=oss-cad-suite/bin/tabbypy3`, no `MAKEFLAGS`, por exemplo): sem
isso, o `verilated.mk` chama o `python3` do sistema com o `PYTHONHOME` do
pacote, herdado do lançador, e ele falha ao carregar a biblioteca padrão.

A AURORA recusa um bundle em que um componente, ou um executável, seja um
symlink para fora dele. E recusa rodar o Verilator se faltar o
`verilator_bin` do pacote: sem ele, o script Perl procuraria um
`verilator_bin` no `PATH`, que no Linux e no macOS inclui `/usr/bin`.

O `dot` do Linux recebe um `fonts.conf` gravado pela AURORA na pasta da
síntese, a partir do modelo que o pacote traz (como fazem os lançadores do
`xdot` e do `gtkwave`), e usa as fontes do pacote.

## 4. O que vem do sistema

| O quê | Por quê | Onde a AURORA procura |
|---|---|---|
| `/bin/bash`, `dirname`, `readlink` (Linux e macOS) | os lançadores do OSS CAD Suite são scripts bash; é o jeito oficial de rodar o pacote | fazem parte do sistema base |
| `libc` (Linux e macOS) | o `aurora` e o YANC são binários nativos ligados à `libc` do sistema; as ferramentas do OSS CAD Suite não, carregam as bibliotecas do pacote | sistema base |
| fontes (macOS e Windows) | os pacotes dessas plataformas não trazem fontes; o `dot` usa as do sistema | as do sistema |
| compilador C++, `make`, Perl (Linux e macOS) | exceção decidida pelo autor: o Verilator compila o modelo em C++, e o OSS CAD Suite não traz compilador. No Windows os três vêm no bundle, com o Verilator | locais fixos: `/usr/bin` (Linux); `/usr/bin` com as Command Line Tools do Xcode (macOS) |
| Quartus Prime (Windows e Linux) | segunda exceção decidida pelo autor: o bitstream das placas Intel sai dele, que é proprietário, tem dezenas de GB e não pode ir no bundle. Só as placas FPGA o usam, um recurso experimental desligado por padrão (`aurora experimental enable fpga`) | `--quartus` ou `AURORA_QUARTUS`; senão `QUARTUS_ROOTDIR`; senão `C:\intelFPGA_lite`, `C:\intelFPGA`, `C:\altera_lite`, `C:\altera` (Windows) e as mesmas na pasta do usuário e em `/opt` (Linux), a versão mais nova ([FPGA.md](FPGA.md)) |
| `taskkill.exe` (Windows) | encerrar a árvore de processos de um passo cancelado ou que passou do prazo (`taskkill /T /F`), só quando o sistema recusa o Job Object do passo, que é o que normalmente a encerra. Só encerra, não executa trabalho; no Linux e no macOS, a AURORA sinaliza o grupo de processos do passo e não roda programa nenhum | `%SystemRoot%\System32` (`C:\Windows\System32` sem `SystemRoot`) |

Sem o compilador, tudo funciona menos a simulação com Verilator, e
`aurora tools` avisa. Para um compilador fora do local padrão, a opção global
`--compiler` ou a variável `AURORA_COMPILER`, que vale para toda chamada. No
Windows ela troca o compilador do bundle pelo de um MSYS2 instalado, só
para desenvolvimento:

```
AURORA_COMPILER=/opt/gcc/bin aurora tools        # Linux, macOS: diretório com perl, make e g++/clang++
aurora tools --compiler D:\msys64              # Windows: raiz de um MSYS2, no lugar do bundle
```

Como deixar a variável definida em cada shell: [CLI.md](CLI.md#de-onde-vêm-as-ferramentas).

Instalar o compilador:

| Sistema | Comando |
|---|---|
| Debian, Ubuntu | `sudo apt install build-essential perl` |
| Fedora | `sudo dnf install gcc-c++ make perl` |
| Arch | `sudo pacman -S --needed base-devel` |
| macOS | `xcode-select --install` |
| Windows | nada: vem com o componente verilator |

## 5. Limitações conhecidas

- **Verilator grava VCD, não FST.** No Linux e no macOS, o FST do Verilator
  compila contra lz4 e zlib, que não vêm no pacote nem fazem parte da
  exceção do compilador. O Windows segue o mesmo formato. O Icarus grava
  FST, e a AURORA troca a extensão do `$dumpfile` numa cópia do testbench
  quando ele pede outra.
- **As versões de Icarus e Verilator diferem entre as plataformas.** No
  Linux e no macOS são os builds de desenvolvimento do OSS CAD Suite
  (Icarus 14 devel, Verilator 5.053 devel); no Windows, as versões estáveis
  do MSYS2 (Icarus 13.0, Verilator 5.050). Os testes rodam nas três.
- **O `dot` do OSS CAD Suite é antigo** (Graphviz 2.43, de 2019). Funciona
  para o esquemático; o do Windows é o 16.1.0.
- **O esquemático depende das fontes no macOS e no Windows.** Lá o `dot`
  mede o texto com as fontes do sistema, e o SVG pode variar de máquina para
  máquina. No Linux usa as do bundle.
- **Esquemático diferente da AURORA.** A AURORA desenha com o `show` do Yosys e
  o `dot`; a AURORA, com o netlistsvg e skins próprios.
- **O YANC é compilado pelo empacotamento, e o surfer-aurora, uma vez por
  commit.** O YANC não publica build de macOS, e o surfer-aurora publica só
  o executável de Windows e o cliente web. O executável do surfer-aurora
  das três plataformas sai do workflow `surfer-aurora.yml`, manual, que o
  compila do commit fixado e o publica numa pré-release
  `surfer-aurora-<versão>` deste repositório; o `bundle.py` baixa e
  confere pelo SHA-256 do `prebuilt` em `bundle/versions.json`, em vez de
  compilar em cada release (uns 14 minutos por plataforma). O cliente web, igual nas três plataformas, vem
  do zip publicado: é ele que a AURORA IDE mostra numa aba, ligado a um
  `surfer-aurora server` da mesma versão (o cliente recusa um servidor com
  outra versão do leitor de ondas).

## 6. Montar um bundle

```
python3 scripts/bundle.py --out dist/toolchain                 # completo, plataforma atual
python3 scripts/bundle.py --out dist/toolchain --only yanc     # parcial, para testes de build
```

O script baixa (uma vez, para `.bundle-cache/`) e confere o SHA-256 de cada
pacote, compila o YANC do commit fixado (e o surfer-aurora, quando não há o
pré-compilado da plataforma em `prebuilt`, ou com `AURORA_BUILD_SURFER`),
separa a parte de
cada componente e grava o `bundle.json`, os `components/<nome>.json` com o
hash de cada executável, e `dist/toolchain.contents.json`, o índice de que
arquivo é de que componente, que os instaladores usam
([RELEASE.md](RELEASE.md)). É o único lugar que baixa um pacote de
ferramenta: em uso, a AURORA só lê o bundle, e o `aurora install` e o
`aurora update` baixam da release da AURORA os pedaços do bundle já montado.

`--platform` monta a divisão de outra plataforma: os pacotes prontos se
dividem em qualquer sistema; o YANC, o surfer-aurora sem pré-compilado e o
Studio com `AURORA_IDE_SRC` precisam compilar para ela.

`--surfer-prebuilt <pasta>` só compila o surfer-aurora e grava na pasta o
pacote pré-compilado da plataforma (`surfer-aurora-<versão>-<plataforma>.tar.gz`,
com o executável e as licenças), que o workflow `surfer-aurora.yml` publica.
Quando o commit do surfer-aurora muda em `bundle/versions.json`, rode o
workflow (`gh workflow run surfer-aurora.yml`) e troque as URLs e os SHA-256
do `prebuilt` pelos do resumo da execução.

No Windows, o pacote msys precisa do SHA-256 da release do aurora-toolchain em
`bundle/versions.json` (o zip em `sha256`, o manifesto em
`manifest_sha256`); sem ele, o `bundle.py` para e diz o que falta. Para
testar um bloco que ainda não foi publicado, `AURORA_MSYS_DIST` aponta para o
`dist/` de um build local do aurora-toolchain (com o `lace-msys-<tag>.zip` e o
`.json`), e o `bundle.py` usa ele no lugar da release.

Precisa, além de Python 3.9+ e `git`:

| Sistema | Para o YANC | Para o surfer-aurora | Para o Studio (só com `AURORA_IDE_SRC`) |
|---|---|---|---|
| Linux | `gcc`, `make`, `flex`, `bison` | `cargo` | `cargo`, Node.js 22, `libwebkit2gtk-4.1-dev` e o resto da [lista do Tauri](https://v2.tauri.app/start/prerequisites/) |
| macOS | Command Line Tools, `flex` e `bison` do Homebrew (`YANC_MAKE_ARGS="BISON=$(brew --prefix bison)/bin/bison FLEX=$(brew --prefix flex)/bin/flex"`) | `cargo` | `cargo`, Node.js 22 |
| Windows | shell MINGW64 do MSYS2 com `mingw-w64-x86_64-gcc`, `make`, `flex`, `bison` | `cargo` | `cargo` (MSVC), Node.js 22 |

O Studio vem da release do aurora-ide na versão do `bundle/versions.json`
(pacote `studio`), pela URL e pelo SHA-256 de cada plataforma em
`prebuilt`. Uma IDE nova é uma release do aurora-ide: troque a versão, as
URLs e os SHA-256 pelos do resumo da execução do `release.yml` de lá. Com
`AURORA_IDE_SRC` apontando para um clone do aurora-ide, o `bundle.py`
compila a IDE desse clone no lugar da release, e a versão dele tem que ser a
do `versions.json`. Para montar sem a IDE, `--only` com os outros
componentes.

O workflow `.github/workflows/installers.yml`, que o `release.yml` chama
numa tag, monta o bundle e os instaladores nas três plataformas e confere o
bundle com `aurora tools --verify`, sem rodar os testes: os instaladores
publicados se testam à mão.

### Atualizar uma versão

1. Trocar a versão e o hash em `bundle/versions.json` (em `packages`). O SHA-256 dos pacotes
   do OSS CAD Suite está no campo `digest` da API de releases do GitHub; o do
   Graphviz, no arquivo `.sha256` ao lado do zip. No Linux e no macOS, o
   bundle acompanha o OSS CAD Suite: atualizar é trocar a data dele. No
   Windows, o Icarus e o Verilator mudam com o pacote msys: publique uma
   release `ucrt64-vN` no aurora-toolchain (o workflow dele monta, testa os
   quatro fluxos e publica) e troque a tag, as duas URLs e os dois hashes,
   que estão no `SHA256SUMS` da release.
2. Mudar `bundle`, o identificador do bundle.
3. No YANC, pôr o `vendor/yanc` no mesmo `commit` de
   `bundle/versions.json` (ou apontar `AURORA_TEST_YANC_SRC` para um fonte
   nesse commit): é dele que `crates/aurora-core/tests/yanc_regression.rs` lê
   os casos de teste do próprio YANC, e com outro commit os testes comparam
   o compilador novo com os casos de teste antigos. A release não roda
   testes: esses rodam à mão.
4. Montar o bundle e rodar os testes
   (`AURORA_TEST_BUNDLE="$PWD/dist/toolchain" cargo test`; ver a seção 7).

Nada no código da AURORA muda com a versão, a menos que um pacote mude de
estrutura (nome de diretório, forma dos lançadores). Se um caminho de
`bundle/components.json` sumir do pacote, ou um binário pedir uma biblioteca
que o pacote não tem, o `bundle.py` falha dizendo qual.

## 7. Testes

```
export AURORA_TEST_BUNDLE="$PWD/dist/toolchain"
cargo test
AURORA_TEST_GUI=1 cargo test --test tools surfer     # abre uma janela do surfer-aurora
```

`AURORA_TEST_BUNDLE` precisa ser um caminho absoluto. O `cargo test` roda
cada teste no diretório do crate (`crates/aurora-core`, `crates/aurora-cli`),
e um caminho relativo como `dist/toolchain` é procurado lá: os testes do
`aurora-core` falham com "AURORA_TEST_BUNDLE não é um bundle válido", mesmo
com o bundle no lugar certo a partir da raiz do repositório. O fonte do YANC
para `yanc_regression` vem de `vendor/yanc`, no commit de
`bundle/versions.json`, ou de `AURORA_TEST_YANC_SRC`.

Sem `AURORA_TEST_BUNDLE`, os testes que dependem de ferramenta avisam e passam.
Com `CI` definido, a ausência é falha.
