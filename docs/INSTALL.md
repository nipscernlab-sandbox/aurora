# Instalar a AURORA

Cada release traz um instalador para o Linux, um para o macOS e dois para o
Windows. O instalador põe a AURORA, a
AURORA IDE e o bundle de ferramentas (as versões exatas, ver
[BUNDLE.md](BUNDLE.md)) numa pasta só, e deixa escolher o que instalar: o
Studio e cada ferramenta do bundle são componentes, e só a linha de comando
é sempre instalada.

| Plataforma | Arquivo | Instalador |
|---|---|---|
| Windows 10 e 11, x64 | `aurora-<versão>-windows-x64-web-setup.exe`, que baixa só os aplicativos marcados, ou `aurora-<versão>-windows-x64-setup.exe`, com o bundle inteiro | assistente (Inno Setup) |
| Linux x64 | `aurora-<versão>-linux-x64.tar.gz` | no terminal (TUI) |
| macOS, Apple Silicon | `aurora-<versão>-darwin-arm64.tar.gz` | no terminal (TUI) |

O `SHA256SUMS` da release tem o hash de cada arquivo que o `release.yml`
publica. O `.vsix` da extensão do VS Code, anexado à mão depois, não está
nele.

## Requisitos

| Sistema | Mínimo | A AURORA IDE pede ainda |
|---|---|---|
| Linux x64 | glibc 2.35: Ubuntu 22.04, Debian 12, Fedora 36, Arch ou mais novos | o webkit2gtk 4.1 e o GTK do sistema |
| macOS, Apple Silicon | macOS 13 | nada |
| Windows x64 | Windows 10 ou 11 | o WebView2, que vem com o Windows 11 e com o Windows 10 atualizado |

O webkit2gtk 4.1 não vem com a AURORA. O GNOME costuma já tê-lo; o KDE
Plasma, o Arch e os ambientes leves, em geral, não:

```sh
sudo apt install libwebkit2gtk-4.1-0     # Debian, Ubuntu, Mint
sudo dnf install webkit2gtk4.1           # Fedora
sudo pacman -S webkit2gtk-4.1            # Arch, Manjaro
```

O instalador confere a biblioteca quando o Studio entra (na instalação ou
no `aurora install studio`). Faltando, ele diz o comando do gerenciador de
pacotes do sistema e, na instalação guiada, pergunta se o roda na hora. Com
`--yes`, só avisa. Aberto pelo menu sem a biblioteca, o Studio mostra numa
janela o que falta e o comando que a instala.

Para compilar o Studio do código-fonte, veja o repositório dele,
[nipscernlab-sandbox/aurora-ide](https://github.com/nipscernlab-sandbox/aurora-ide)
(o `docs/DEVELOPMENT.md` de lá tem as dependências do Tauri por sistema).

O mínimo de Linux é o da distribuição em que o CI compila (`ubuntu-22.04`);
o de macOS, o dos binários do OSS CAD Suite. O instalador avisa quando o
WebView do Studio falta, sem deixar de instalar.

## Quem tem o Lace

A AURORA 1.0.0 é a versão seguinte ao Lace 0.9.0, com outro nome, outra
pasta e outro executável. Para passar, basta o `lace update`, ou a
atualização do Lace Studio: a release da AURORA publica os instaladores
dela também com os nomes que o Lace procura, e eles trocam o Lace pela
AURORA, com os mesmos aplicativos.

- **Linux e macOS:** a AURORA vai para a pasta dela (`~/.local/share/aurora`,
  ou `/opt/aurora` como root), com o atalho `aurora` onde estava o `lace`,
  e o Lace sai pelo desinstalador dele. Um Lace numa pasta com outro nome
  dá lugar a uma AURORA na pasta padrão.
- **Windows:** abre o assistente da AURORA, com os aplicativos do Lace
  marcados e a tarefa **Uninstall Lace** (Desinstalar o Lace), que o
  desinstala no fim. Se a atualização veio do Lace Studio, a AURORA IDE abre
  quando o assistente termina.

Instalando a AURORA à parte, pelo [comando de um passo](#em-um-comando) ou
pelo instalador, o assistente do Windows também acha o Lace e oferece a
mesma tarefa; no Linux e no macOS, tire o Lace depois com `lace uninstall`.

A AURORA aproveita sozinha os projetos, cuja pasta `.lace/` vira `.aurora8/`
na primeira abertura, com o que tinha dentro, e as preferências do Lace
Studio, que a AURORA IDE copia na primeira vez que abre. Começam de novo:

- a pasta de exercícios do `lace learn`, que a AURORA não reconhece: comece
  outra com `aurora learn init`;
- as variáveis `LACE_*`, que passam a ser `AURORA_*`, e os recursos
  experimentais, que se ligam de novo com `aurora experimental`;
- a extensão do VS Code, que é outra (AURORA), com as configurações
  `aurora.*` no lugar de `lace.*`; a do Lace sai pelo próprio VS Code;
- uma placa compilada pelo Lace, que precisa ser compilada de novo antes de
  gravar.

## Quem tem o Solar 0.1.0

A 0.1.0 saiu com o nome Solar. O Lace 0.2.0 foi a versão seguinte, com outro
nome, outra pasta e outro executável, e não trocava o Solar sozinho: os dois
ficam instalados lado a lado até o Solar sair.

- **Linux e macOS:** `~/.local/share/solar/uninstall.sh` (ou
  `sudo /opt/solar/uninstall.sh`), e instalar a AURORA.
- **Windows:** Configurações > Aplicativos > Solar > Desinstalar, que tira
  também a entrada do Solar no PATH, e instalar a AURORA.

A configuração do Solar (`~/.config/solar/config.json`) nem o Lace nem a
AURORA leem; ela pode ser apagada à mão. Os projetos (`.spf`) não mudam.

## Em um comando

```sh
curl -fsSL https://raw.githubusercontent.com/nipscernlab-sandbox/aurora/main/install.sh | sh     # Linux, macOS
```

```powershell
irm https://raw.githubusercontent.com/nipscernlab-sandbox/aurora/main/install.ps1 | iex          # Windows, PowerShell
```

Os dois scripts, `install.sh` e `install.ps1` na raiz do repositório, fazem
a mesma coisa: descobrem a última release, baixam o instalador da
plataforma, conferem o SHA-256 com o `SHA256SUMS` da release e abrem o
instalador. Os dois mostram o progresso do download: o `install.sh`, a
barra do `curl`; o `install.ps1`, uma linha com a barra, a porcentagem, os
MiB, a velocidade e o tempo que falta, que encolhe para caber na janela. O
`install.sh` roda em qualquer shell, porque é executado pelo `sh`; a
instalação guiada lê o teclado pelo terminal mesmo com o `| sh`. Ele baixa
para `~/.cache`, ou para `/var/tmp` como root (há distribuições que montam o `/tmp` sem permissão de
execução) e apaga o que baixou no fim.

| O quê | Linux, macOS | Windows |
|---|---|---|
| versão fixa | `... \| AURORA_VERSION=1.0.0 sh` | `$env:AURORA_VERSION = "1.0.0"` antes do `irm` |
| sem perguntas | `... \| sh -s -- --yes` | `$env:AURORA_SETUP_ARGS = "/VERYSILENT /SUPPRESSMSGBOXES /CURRENTUSER /TYPE=sapho /TASKS=path"` antes do `irm` |
| outras opções do instalador | `... \| sh -s -- --yes --profile verilog --prefix /opt/aurora` | os parâmetros do Inno Setup em `AURORA_SETUP_ARGS` |
| para todos os usuários | `... \| sudo sh` (vai para `/opt/aurora`) | o assistente pergunta |

## Idioma, perfis e componentes

O assistente do Windows começa perguntando o idioma, English ou Português
(Brasil), com o do Windows já escolhido. Depois vem o perfil: um conjunto de
aplicativos para um tipo de trabalho. A página do perfil mostra a lista
inteira dos aplicativos, com os que ele instala marcados, e diz para quem
ele serve.

| Perfil | Para quem | Aplicativos |
|---|---|---|
| **SAPHO** (recomendado, o padrão) | quem constrói processadores SAPHO e os programa em C±, com as ferramentas de Verilog em volta | todos |
| **Verilog** | quem projeta em Verilog sem processadores SAPHO: módulos, testbenches em Verilog ou cocotb, simulação no Icarus ou no Verilator, síntese e placas FPGA | todos menos o YANC |
| **Custom** (avançada; no assistente em português, Personalizada) | uso avançado, para quem sabe de quais componentes precisa | os marcados |

Com um perfil, a lista fica travada: ela só mostra o que vai ser instalado.
Só o Custom deixa marcar e desmarcar. A AURORA (CLI & Core), a linha de
comando e o Core, vem sempre marcado e travado, em qualquer perfil. Na
instalação guiada do Linux e do macOS, o perfil sob o cursor mostra cada
aplicativo com `[x]` ou `[ ]`, e o Custom abre a lista para marcar.

| Componente | O que é | SAPHO | Verilog |
|---|---|---|---|
| AURORA (CLI & Core) | a linha de comando e o Core; sempre instalado, porque é ele que instala, atualiza e desinstala (`aurora install`, `aurora update`, `aurora uninstall`) | sim | sim |
| AURORA IDE | o ambiente gráfico: editor, compilação, simulação, ondas; atalho no menu de aplicativos | sim | sim |
| YANC | compiladores C± e C do SAPHO e a biblioteca SAPHO | sim | não |
| Icarus Verilog | simulador Verilog, o padrão da AURORA | sim | sim |
| Verilator | simulador compilado; no Linux e no macOS precisa de compilador C++, `make` e Perl do sistema, no Windows vem com eles | sim | sim |
| cocotb | testbenches em Python, com o Python que os roda; precisa do Icarus. A AURORA roda os testes no Icarus (`aurora sim test_x.py`, ou Wave e Rápida no Studio) e, com o componente Verilator, no Verilator (`aurora sim --verilator`) | sim | sim |
| Yosys | síntese e esquemático; lê as portas para o testbench-modelo | sim | sim |
| Graphviz (dot) | desenho do esquemático; precisa do Yosys | sim | sim |
| RTL viewer | o esquemático interativo do netlist da síntese no navegador (`aurora synth --open`); precisa do Yosys | sim | sim |
| surfer-aurora | visualizador de formas de onda (o fork do Surfer da AURORA) | sim | sim |
| openFPGALoader | grava as placas FPGA pelo cabo USB delas ([FPGA.md](FPGA.md)); no Linux, o acesso ao cabo pede uma regra do udev | sim | sim |
| AURORA Learn | os exercícios de Verilog do `aurora learn` e da vista Exercícios do Studio ([LEARN.md](LEARN.md)); precisa do Icarus | sim | sim |

O Verilator, no Linux e no macOS, só funciona com o compilador do sistema
instalado (ver [BUNDLE.md](BUNDLE.md), seção 4); no Windows ele traz o g++,
o `make` e o Perl, uns 630 MiB. Quem não quer o Verilator usa o Custom.
Marcar o Graphviz ou o RTL viewer marca o Yosys; desmarcar o Yosys desmarca
os dois. Do mesmo jeito, marcar o cocotb ou o AURORA Learn marca o Icarus.

Os perfis ficam em `bundle/components.json` (`profiles`), com o nome, o
texto de cada um e os componentes; o primeiro é o padrão e o recomendado. Os
textos em português ficam em `translations`, ao lado. Um perfil traz o
que cada componente dele exige, e o `bundle.py` recusa um que não traga.

A AURORA IDE fica no bundle, em `toolchain/studio/`, e usa o bundle em que
está. O atalho
dele vai para o menu de aplicativos: o `aurora-ide.desktop` em
`~/.local/share/applications` no Linux (que abre o `aurora-ide-launch`, ao
lado do Studio: ele confere as bibliotecas do sistema antes de abrir), o
`AURORA IDE.app` em
`~/Applications` no macOS (um symlink para o do bundle), o menu Iniciar no
Windows.

Uma ferramenta de componente não instalado dá erro claro quando usada:

```
Error: Component verilator is not installed
  Install it with: aurora install verilator
```

`aurora tools` mostra o que está instalado e o que não está.

Tamanho instalado no Linux, de referência (0.2.0, com menos componentes que
hoje): 378 MiB com tudo, dos quais 16 MiB são da AURORA IDE; o instalador
(`.tar.gz`) tem 106 MB. Icarus, Verilator, Yosys e Graphviz dividem bibliotecas
do OSS CAD Suite, então a soma dos tamanhos de cada um é maior que o total.
`./install --list` mostra o tamanho de cada perfil e de cada componente na
plataforma, e a página de componentes do assistente, o da seleção.

## Windows

1. Rode o `irm ... | iex` acima, ou baixe e rode um dos dois assistentes da
   página da release:
   - `aurora-<versão>-windows-x64-web-setup.exe` (desde a 0.7.0, alguns MB):
     ao clicar em Instalar, baixa da release só os aplicativos marcados, com
     a barra de cada download, confere cada um pelo SHA-256 e os extrai.
     É o que o `irm ... | iex` usa;
   - `aurora-<versão>-windows-x64-setup.exe`: o bundle inteiro dentro dele,
     para instalar sem rede (o `irm ... | iex` o usa com
     `$env:AURORA_FULL_SETUP = "1"`).
2. Escolha instalar só para você (padrão, sem administrador, em
   `%LOCALAPPDATA%\Programs\AURORA`) ou para todos os usuários (pede
   administrador, em `C:\AURORA`). Para todos os usuários, a pasta fica com
   permissões próprias: os administradores e o sistema escrevem nela, os
   outros usuários só leem e executam. Sem isso, ela herdaria de `C:\` a
   escrita de qualquer usuário, e o `aurora.exe`, no PATH do sistema, poderia
   ser trocado por qualquer um.
3. Escolha o idioma do assistente.
4. Na página **Choose a profile** (Escolha um perfil), escolha o perfil na
   caixa: a lista embaixo mostra, travada, os aplicativos que ele instala.
   Para escolher à mão, use o **Custom** (Personalizada), para uso avançado.
5. Deixe marcada a tarefa **Add AURORA to PATH** para usar `aurora` em
   qualquer terminal novo. Com o Studio, o assistente põe o atalho no menu
   Iniciar e oferece um na área de trabalho.
6. Com o Lace instalado, o assistente avisa na primeira página, marca os
   aplicativos dele e deixa marcada a tarefa **Uninstall Lace**
   (Desinstalar o Lace), que o desinstala no fim, depois de pôr a AURORA no
   PATH. Apagar o bundle do Lace pode levar alguns minutos, e o assistente
   mostra o andamento até ele sair ([Quem tem o Lace](#quem-tem-o-lace)).

A pasta padrão não tem espaço no caminho de propósito: o `make` usado pelo
Verilator não aceita espaços, e `C:\Program Files` tem um. O assistente avisa
se a pasta escolhida tiver. Com o Studio marcado e sem o WebView2 no
computador, ele avisa antes de instalar.

O assistente recusa uma pasta que já tem uma `toolchain` que não é de uma
instalação da AURORA (sem `toolchain\bundle.json` nem `bin\aurora.exe`):
instalar trocaria essa pasta inteira, sem passar pela lixeira.

Numa pasta que já tem a AURORA, a primeira página diz de que versão para qual
ele atualiza, e os aplicativos que a pasta tem vêm marcados, inclusive os
que o `aurora install` acrescentou: uma seleção que não é a de nenhum perfil
fica no Custom. O mesmo vale numa instalação silenciosa sem `/COMPONENTS`.

O assistente web baixa do GitHub; `/MIRROR=<url>` o faz baixar de um
espelho do laboratório, com uma pasta `v<versão>/` por release, como o
`AURORA_RELEASE_URL`. Uma falha no download para o assistente na página de
instalação, sem ter mudado nada; uma falha ao extrair, antes de copiar
qualquer arquivo.

O Verilator no Windows vem com o g++, o `make` e o Perl que ele usa (o bloco
MSYS2 do aurora-toolchain): não é preciso instalar o MSYS2.

**Instalar aplicativos do bundle:** `aurora install` abre no terminal a lista
dos aplicativos do bundle (YANC, Icarus, Verilator, cocotb, Yosys, Graphviz,
surfer-aurora, openFPGALoader, AURORA IDE, AURORA Learn, RTL viewer), com os instalados marcados e travados; marque os novos com
`Space` e confirme. `aurora install verilator` instala direto, sem a lista,
com o que o aplicativo exige (`graphviz` traz o Yosys). A AURORA não é
reinstalado: só os aplicativos marcados são baixados e extraídos em
`toolchain/`, e uma falha no meio desfaz o que entrou.

Os aplicativos vêm da release desta versão da AURORA no GitHub, que publica
o bundle em pedaços (`aurora-<versão>-<plataforma>-<pedaço>`), conferidos
pelo `SHA256SUMS` da release. Sem rede, `aurora install --from` usa um
instalador da AURORA no disco (a pasta, o `.tar.gz` da release ou o
`payload/` dele, ou uma pasta com os arquivos que a release publica);
`AURORA_RELEASE_URL` aponta para um espelho do laboratório, com uma pasta
`v<versão>/` por release e, se quiser, um arquivo `latest` com a versão
mais nova, que o `aurora update` lê no lugar do GitHub. O espelho é
`https://`, ou `file://` numa pasta do disco: o `SHA256SUMS` que confere
os downloads vem dele também, e a AURORA recusa `http://`. O bundle de lá
precisa ser o instalado; outro bundle é atualizar (`aurora update`).

No Windows, o Studio instalado pelo `aurora install studio` não ganha atalho
no menu Iniciar (o atalho é do assistente): abra
`<pasta>\toolchain\studio\aurora-ide.exe`, ou rode o assistente de novo
com o Studio marcado.

O desinstalador do Windows apaga o `toolchain/` inteiro, com o que o
`aurora install` pôs. Rodar o assistente de novo à mão também refaz o
`toolchain/` com a seleção dele; os aplicativos que a pasta tem, inclusive
os que o `aurora install` acrescentou, já vêm marcados, e o que for
desmarcado sai.

**Atualizar:** a AURORA IDE procura versão nova ao abrir, oferece a
atualização e reinicia na versão nova; o `aurora`, num terminal, avisa no fim
de um comando, uma vez por dia ([CLI.md](CLI.md), "Bundle").
`aurora update --check` compara as versões; `aurora update`
atualiza por componentes (abaixo, em [Atualizar por componentes](#atualizar-por-componentes)),
sem abrir o assistente e sem tirar o que o `aurora install` acrescentou, e a
lista de programas do Windows passa a mostrar a versão nova. Numa
instalação anterior à 0.7.0, ou com `aurora update --full`, ele baixa o
assistente da release nova, confere o SHA-256 e o abre: ele lembra a pasta,
e os componentes instalados vêm marcados, os do assistente e os que o
`aurora install` acrescentou.

**Mudar os componentes:** rode o instalador de novo. Ele lembra a pasta e a
seleção anteriores; o que for desmarcado é removido.

**Remover:** `aurora uninstall`, Configurações > Aplicativos > AURORA, ou
`<pasta>\unins000.exe`; os três abrem o mesmo desinstalador. Sai a AURORA,
o bundle inteiro e a entrada do PATH. `aurora uninstall --yes` desinstala sem
perguntar.

**Sem interação** (scripts, laboratórios):

```
aurora-1.0.0-windows-x64-setup.exe /VERYSILENT /SUPPRESSMSGBOXES /CURRENTUSER /TYPE=sapho /TASKS=path
aurora-1.0.0-windows-x64-setup.exe /VERYSILENT /SUPPRESSMSGBOXES /CURRENTUSER /LANG=pt /TYPE=verilog /TASKS=path
aurora-1.0.0-windows-x64-setup.exe /VERYSILENT /SUPPRESSMSGBOXES /CURRENTUSER /TYPE=avancada /COMPONENTS="aurora,yanc,icarus,verilator,studio" /DIR=D:\AURORA
```

O idioma (`/LANG=`) é `en` ou `pt`; sem ele, o do Windows. Os tipos
(`/TYPE=`) são os perfis, `sapho` e `verilog`, e
`avancada`, o Custom, que guarda o nome da instalação Advanced de antes dos
perfis. O `/TYPE=recomendada` de antes cai no perfil padrão.
Nos parâmetros do Inno Setup, os nomes dos componentes são `aurora`, `yanc`, `icarus`,
`icarus\cocotb`, `icarus\aurora_learn`, `verilator`, `yosys`,
`yosys\graphviz`, `yosys\rtl_viewer`, `surfer_aurora`, `openfpgaloader` e `studio`; as
tarefas, `path`, `studioicon` (o atalho do Studio na área de trabalho) e
`removelace` (desinstalar o Lace, quando ele está instalado). Um `/TASKS=`
marca só as tarefas da lista.

O instalador não é assinado: o SmartScreen pode avisar "Editor
desconhecido" na primeira vez (Mais informações > Executar assim mesmo).

## Linux e macOS

Pelo `install.sh` (acima), ou à mão, com o arquivo da release:

```
curl -fLO https://github.com/nipscernlab-sandbox/aurora/releases/download/v1.0.0/aurora-1.0.0-linux-x64.tar.gz
tar xzf aurora-1.0.0-linux-x64.tar.gz && ./aurora-1.0.0-linux-x64/install
```

A instalação guiada no terminal tem estas telas:

1. boas-vindas (avisa se já há uma instalação na pasta padrão);
2. perfil: **SAPHO** (recomendado), **Verilog** ou **Custom** (avançada); o perfil
   sob o cursor mostra para quem é e todos os aplicativos, com `[x]` nos
   que ele instala;
3. componentes, só no Custom: `Space` marca e desmarca; o Verilator mostra
   se o compilador do sistema foi encontrado;
4. pasta da instalação e atalho;
5. resumo;
6. progresso e resultado, com a conferência dos hashes dos executáveis.

`Esc` volta uma tela, `Ctrl+C` sai sem mudar nada; durante a extração, ele
não interrompe.

| | Usuário comum | root (`sudo ./install`, ou `curl ... \| sudo sh`) |
|---|---|---|
| Pasta | `~/.local/share/aurora` | `/opt/aurora` |
| Atalho | `~/.local/bin/aurora` | `/usr/local/bin/aurora` |
| Studio no menu (Linux) | `~/.local/share/applications/aurora-ide.desktop` | `/usr/local/share/applications/aurora-ide.desktop` |
| Studio no menu (macOS) | `~/Applications/AURORA IDE.app` | `/Applications/AURORA IDE.app` |

Se a pasta do atalho não estiver no `PATH`, o instalador diz a linha para
pôr no `~/.bashrc` ou `~/.zshrc`, e o comando do fish
(`fish_add_path ~/.local/bin`). No macOS, `~/.local/bin` não está no `PATH`
por padrão.

O instalador só escreve numa pasta nova, vazia ou com uma instalação do
AURORA. Ele extrai tudo numa pasta provisória, confere o hash de cada
executável com a própria AURORA e só então troca a instalação anterior: uma
falha no meio não estraga o que já estava instalado.

**Sem interação:**

```
./install --list                                   # perfis, componentes e tamanhos
./install --yes                                    # o perfil padrão (SAPHO)
./install --yes --profile verilog                  # outro perfil
./install --yes --components yanc,icarus,verilator # Custom
./install --yes --prefix /opt/aurora --no-link
```

Um componente que exige outro traz o outro junto (`--components graphviz`
instala também o Yosys, e avisa).

**Instalar aplicativos do bundle:** `aurora install` abre no terminal a lista
dos aplicativos do bundle (YANC, Icarus, Verilator, cocotb, Yosys, Graphviz,
surfer-aurora, openFPGALoader, AURORA IDE, AURORA Learn, RTL viewer), com os instalados marcados e travados; marque os novos com
`Space` e confirme. `aurora install verilator` instala direto, sem a lista,
com o que o aplicativo exige (`graphviz` traz o Yosys). A AURORA não é
reinstalado: só os aplicativos marcados são baixados e extraídos em
`toolchain/`, e uma falha no meio desfaz o que entrou. O
`aurora install studio` também cria o atalho do Studio no menu.

Os aplicativos vêm da release desta versão da AURORA no GitHub, que publica
o bundle em pedaços (`aurora-<versão>-<plataforma>-<pedaço>`), conferidos
pelo `SHA256SUMS` da release. Sem rede, `aurora install --from` usa um
instalador da AURORA no disco (a pasta, o `.tar.gz` da release ou o
`payload/` dele, ou uma pasta com os arquivos que a release publica);
`AURORA_RELEASE_URL` aponta para um espelho do laboratório, com uma pasta
`v<versão>/` por release e, se quiser, um arquivo `latest` com a versão
mais nova, que o `aurora update` lê no lugar do GitHub. O espelho é
`https://`, ou `file://` numa pasta do disco: o `SHA256SUMS` que confere
os downloads vem dele também, e a AURORA recusa `http://`. O bundle de lá
precisa ser o instalado; outro bundle é atualizar (`aurora update`).

**Atualizar:** a AURORA IDE procura versão nova ao abrir, oferece a
atualização e reinicia na versão nova; o `aurora`, num terminal, avisa no fim
de um comando, uma vez por dia ([CLI.md](CLI.md), "Bundle").
`aurora update --check` mostra se há AURORA mais nova e,
para cada aplicativo, a versão instalada, a do bundle da última release e a
última upstream. `aurora update` atualiza por componentes (abaixo). Numa
instalação anterior à 0.7.0, ou com `aurora update --full`, ele reinstala
pela release nova, com os mesmos aplicativos, a mesma pasta e o mesmo
atalho. Uma ferramenta mais nova
upstream não é instalada sozinha: o bundle compila o YANC e o surfer-aurora e
divide o OSS CAD Suite por ferramenta, então cada versão nova passa pelo
`bundle.py` e pelo CI e chega numa release nova da AURORA.

**Mudar os componentes:** rode o instalador de novo na mesma pasta; a
instalação é trocada pela nova seleção. Com um perfil (e com
`./install --yes` sem `--components`), os aplicativos que a instalação já
tem ficam, junto dos do perfil; para tirar algum, use o Custom.

**Remover:** `aurora uninstall` (pergunta antes; `--yes` não pergunta) ou
`<pasta>/uninstall.sh`. Sai a AURORA, o bundle inteiro (`toolchain/`), o
atalho, o do Studio no menu e a pasta; uma instalação do sistema (`/opt/aurora`) pede
`sudo aurora uninstall`.

O que não sai, porque não é da instalação: os projetos, com o `.aurora8/` de
cada um (intermediários e ondas, que se apagam à mão), e o histórico de
arquivos do surfer-aurora (`~/.local/share/surfer` no Linux), que o Surfer e
a AURORA também usam, as preferências do Studio, o `experimental.json` do
`aurora experimental` e o `update-check.json` do aviso de versão nova. Um
`~/.config/aurora/config.json` de um build de desenvolvimento anterior à
0.2.0 (o `aurora config`, que saiu), que nenhuma versão lê, sai junto com
`aurora uninstall`.

**Verilator:** precisa de `g++` (ou `clang++`), `make` e `perl` do sistema:
`sudo apt install build-essential perl` (Debian, Ubuntu),
`sudo dnf install gcc-c++ make perl` (Fedora), `sudo pacman -S --needed
base-devel` (Arch), `xcode-select --install` (macOS).

**macOS e o Gatekeeper:** o instalador não é assinado nem notarizado. Um
`.tar.gz` baixado pelo navegador recebe a marca de quarentena, e o macOS
recusa abrir o `install` ("desenvolvedor não pode ser verificado"). O
`install.sh` e o `curl -fLO` baixam pelo `curl`, que não põe a marca; se já
foi baixado pelo navegador:

```
xattr -dr com.apple.quarantine aurora-1.0.0-darwin-arm64
```

Isto não foi verificado num Mac: é o comportamento documentado do
Gatekeeper para binários sem assinatura. Os arquivos que o instalador extrai
não recebem a marca.

## Atualizar por componentes

Desde a 0.7.0, cada instalação guarda o manifesto de arquivos da versão
dela (`toolchain/files.json`): o SHA-256 de cada arquivo que o instalador
pôs, agrupado pelos pedaços em que a release publica o bundle. A release
publica também o seu (`aurora-<versão>-<plataforma>-files.json`). O
`aurora update` compara os dois, para os aplicativos instalados:

```
AURORA 0.7.0 to 0.8.0 by components: AURORA, AURORA IDE, AURORA Learn
3 of 15 chunks to download (14 MiB), 412 files in, 3 out
Update AURORA 0.7.0 to 0.8.0 in /home/ana/.local/share/aurora? [y/N] y
  [1/3] AURORA  [##############################] 100%  2.3/2.3 MiB
  [2/3] AURORA IDE  [#############.................]  45%  5.1/11.3 MiB
```

Só os pedaços com algum arquivo novo ou mudado são baixados, conferidos
pelo `SHA256SUMS` da release, e dentro deles só esses arquivos são
extraídos, numa pasta provisória (`.instalando-<pid>`), e conferidos com o
manifesto novo. Depois os arquivos são trocados no lugar, os antigos vão
para `.antigo-<pid>`, os que a versão nova não tem saem, e a AURORA confere
os executáveis do bundle. Uma falha em qualquer passo devolve os arquivos
antigos, e a instalação fica como estava; aí o `aurora update` passa para o
instalador inteiro.

Antes de trocar, a AURORA grava o que vai fazer num diário
(`.atualizando.json`, na pasta da instalação). Se o processo morre no meio
da troca (o terminal fechado, falta de energia), o próximo `aurora update` ou
`aurora install` desfaz o que ficou feito com o diário: os arquivos antigos
voltam, os novos saem, e ele avisa e pede para rodar o `aurora update` de
novo. Uma trava (`.instalacao.lock`) impede duas instalações ou
atualizações ao mesmo tempo na mesma pasta. A AURORA IDE não fecha
enquanto uma atualização ou instalação dela está rodando.

O próprio `aurora` é um dos arquivos trocados. No Windows, um executável em
uso pode ser renomeado, mas não apagado: o `aurora.exe` antigo fica na pasta
`.antigo-<pid>` até o próximo `aurora install` ou `aurora update`. Se a AURORA
Studio estiver aberto durante a atualização, feche e abra de novo para usar
a versão nova.

Um aplicativo que a versão nova passa a exigir entra junto; um que ela não
tem mais sai. Um arquivo apagado à mão da instalação volta no próximo
`aurora update`; com o instalador da mesma versão no disco,
`aurora update --from` repara a instalação sem rede.

Sem o manifesto (uma instalação anterior à 0.7.0), com `--full`, ou se a
release nova não o publica, o `aurora update` usa o instalador inteiro, como
antes. A primeira atualização de uma instalação 0.6 para a 0.7 é assim; a
partir da 0.7, vai por componentes.

## Depois de instalar

```
aurora tools --verify        # o bundle, as versões e a conferência dos hashes
aurora new meu_projeto
cd meu_projeto
aurora add top.v             # Verilog: cria o módulo e o registra como topo
aurora proc add filtro       # SAPHO: cria o processador filtro
```

Os dois fluxos, Verilog e SAPHO, completos estão em
[CLI.md, Primeiro uso](CLI.md#primeiro-uso).

## O que fica na pasta

```
<pasta>/
  bin/aurora[.exe]
  toolchain/            o bundle, só com os componentes escolhidos
    bundle.json
    components/<nome>.json
    files.json          o SHA-256 de cada arquivo instalado (desde a 0.7.0)
    yanc/ oss-cad-suite/ surfer-aurora/ graphviz/ msys/ studio/ aurora-learn/ rtl-viewer/
  install.json          (Linux, macOS) o que foi instalado
  uninstall.sh          (Linux, macOS)
  unins000.exe          (Windows)
```
