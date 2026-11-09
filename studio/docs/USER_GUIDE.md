# Guia do usuário

Como usar a AURORA IDE: a janela, os dois fluxos de trabalho (Verilog e
processador SAPHO), o que aparece no INFO e nos terminais, os atalhos e as
preferências. Quem vem da AURORA encontra os mesmos botões com os mesmos
nomes e teclas (C± F6, Verilog F7, Wave F8, RTLViewer F10).

Sumário:

1. [Antes de começar](#1-antes-de-começar)
2. [A janela](#2-a-janela)
3. [Projeto Verilog](#3-projeto-verilog)
4. [Processador SAPHO](#4-processador-sapho)
5. [Os botões do fluxo](#5-os-botões-do-fluxo)
6. [INFO, terminais, problemas e shell](#6-info-terminais-problemas-e-shell)
7. [Esquemático, síntese e relatórios](#7-esquemático-síntese-e-relatórios)
8. [Ferramentas da AURORA](#8-ferramentas-da-aurora)
9. [Preferências](#9-preferências)
10. [Atalhos](#10-atalhos)
11. [Problemas comuns](#11-problemas-comuns)

---

## 1. Antes de começar

O Studio usa as ferramentas da AURORA, e só elas: precisa da AURORA instalada
com o bundle ([instalação da AURORA](https://github.com/ART3121/lace#instalar)).
Ele acha a instalação sozinho; o estado aparece na tela inicial e na barra de
status (`Bundle 2026.09.29`, ou `AURORA não encontrada`). Se a AURORA estiver
numa pasta fora do padrão, indique-a em Preferências > Bundle da AURORA.

Os projetos são os `.spf` da AURORA: um projeto da AURORA 7 abre na IDE, e
o que a IDE grava continua legível pela AURORA 7. Os temporários ficam em
`.aurora8/`; a `.aurora/` é da AURORA 7, e a IDE não mexe nela.

**Abertura.** Enquanto o Studio abre, uma janela pequena mostra o logo do
AURORA sendo desenhado, com um sinal que corre pelos fios, e o que está
carregando. Ela fica pelo menos uns 3 segundos, o tempo da animação, e a
janela principal aparece quando a AURORA foi achada e o último projeto
reabriu, ou depois de 8 segundos, o que vier antes; aí a pequena fecha.

**Tour pela interface.** Na primeira vez que o Studio abre num computador,
com o tema Boreal, o idioma do sistema e o layout Padrão, ele começa um tour
pela janela: tudo escurece, e uma parte de cada vez fica acesa, com um
cartão que diz o que ela faz. A parte acesa continua funcionando: dá para
clicar nela, e alguns passos pedem uma tarefa (abrir o menu Fluxo, trocar
de vista pela barra de atividades, abrir Problemas no cabeçalho do painel,
esconder o painel e trazê-lo de volta com Ctrl+J, abrir a paleta) e seguem
sozinhos quando ela é feita. Pular
passa uma tarefa, as setas do teclado avançam e voltam, e Esc sai. O tour
volta em Ajuda > Tour pela interface e na tela inicial. Um Studio atualizado
de uma versão anterior não abre o tour sozinho.

## 2. A janela

```
 título       Arquivo Editar Exibir Projeto Fluxo Ferramentas Ajuda   projeto - AURORA IDE    _ □ X
 ferramentas  Novo Abrir Salvar | Alvo [Projeto] | C± Verilog | Wave Rápida | Onda Sinais | RTLViewer | Parar
 ┌──┬────────────────┬──────────────────────────────────┬──────────────┐
 │  │ barra lateral  │ abas: arquivos e vistas          │ barra lateral│
 │at│ esquerda       │                                  │ direita      │
 │iv│ (explorador,   │                                  │ (vazia no    │
 │id│  fluxo, busca, ├──────────────────────────────────┤  Padrão)     │
 │ad│  relatórios)   │ painel: INFO, Terminal,          │              │
 │es│                │ Problemas                        │              │
 └──┴────────────────┴──────────────────────────────────┴──────────────┘
 status       projeto, topo, testbench | operação ou último resultado | problemas, cursor, simulador, bundle, regiões
```

Esse é o layout Padrão. Cada vista pode ir para outra região, e cada barra
e item pode ser escondido (seção 2.4).

- **Barra de título**: os menus, o nome do projeto e os botões de
  minimizar, maximizar e fechar numa faixa só. Arrastar a faixa move a
  janela, e dois cliques nela maximizam e restauram. No macOS, os botões
  coloridos do sistema ficam no canto esquerdo da faixa. Em tela cheia a
  faixa some, e os menus ficam numa linha própria.
- **Barra de ferramentas**: no Padrão, uma linha própria com os botões de
  arquivo, o Alvo e os do fluxo lado a lado, com um separador entre
  compilar, simular, a onda e o hardware. Há outros dois modelos, com o
  botão Executar na barra de título ou no editor (seção 2.4).
- **Barra de atividades**, à esquerda: as vistas da barra lateral esquerda
  em cima; Ferramentas da AURORA e Preferências embaixo. Clicar na vista
  aberta esconde a barra lateral (Ctrl+B faz o mesmo).
- **Explorador**, com três modos (seção 2.1): *Fontes*, *Hierarquia* e
  *Arquivos*. Os botões de cada modo ficam na linha do título, à direita.
- **Fluxo**: as etapas do projeto em ordem, como o Flow Navigator do
  Vivado, cada uma com um ponto que diz como terminou a última execução.
- **Abas**: um clique no explorador abre o arquivo numa aba provisória
  (título em itálico), que a próxima abertura troca; editar ou dar duplo
  clique a fixa. Vistas como o esquemático e as preferências também abrem
  em abas. O editor se divide em até três grupos lado a lado (seção 2.2).
- **Painel**: o INFO, com o que as operações escrevem, o Terminal e
  Problemas, embaixo do editor ou à direita dele. Ctrl+J esconde e mostra; o botão de maximizar dá
  ao painel o lugar do editor.
- **Barra lateral direita**: vazia no Padrão; recebe as vistas que você
  mover para ela e, com o Assistente de IA ligado (seção 9), a vista
  Assistente: Exibir > Assistente a abre ali, e Exibir > Assistente em aba,
  numa aba do editor. Ctrl+Alt+B esconde e mostra.
- **Barra de status**: o que está sendo feito agora e há quanto tempo, ou
  como terminou a última operação. Clicar nos contadores de problemas abre
  o painel Problemas; clicar no bundle abre as Ferramentas da AURORA. No
  fim, três botões mostram e escondem a barra lateral esquerda, o painel e
  a barra lateral direita, como Ctrl+B, Ctrl+J e Ctrl+Alt+B; o da região
  escondida fica apagado.

Todo divisor entre as áreas pode ser arrastado, e o tamanho fica guardado.
Duplo clique num divisor volta ao tamanho do layout em uso.

### 2.1 Explorador

**Fontes** mostra o projeto como a AURORA o vê:

- *Módulos*: os sintetizáveis registrados, com o topo marcado pelo nome do
  módulo, e o Verilog que o build de cada processador gerou
  (`<proc>/<proc>/<proc>.v`), marcado como **Gerado**.
- *Testbenches*: os registrados, com o simulado marcado, e o testbench que o
  YANC gerou para cada processador (`<proc>/<proc>/<proc>_tb.v`). O
  menu de um testbench tem **Marcar como o testbench simulado**, que só
  marca: quem simula é a Wave (F8) ou a Rápida (F9). No gerado, que ainda
  não está no `.spf`, o mesmo item antes o registra como testbench.

O **+** no cabeçalho de *Módulos*, *Testbenches* e *Processadores SAPHO*
cria um módulo, um testbench ou um processador.
- *Processadores SAPHO*: cada um com o **Programa** (o fonte e o assembly
  gerado), as **Memórias** (`.mif`), a **Simulação** (entradas, saídas e a
  onda) e, recolhidos, os **Intermediários do YANC** (logs e traduções da
  pasta temporária).
- *Fora do projeto*: os `.v` da pasta que ainda não estão no `.spf`. O
  **+** que aparece ao passar o ponteiro (ou com a linha em foco) registra o
  arquivo, como o Adicionar ao projeto do menu de contexto: um testbench vai
  para Testbenches, o resto para Módulos. O botão de lista com **+**, no
  cabeçalho da seção, registra todos de uma vez.

**Hierarquia** mostra a árvore de instâncias como o Icarus a elabora: em
*Design*, os módulos que ninguém instancia (o topo, marcado) com tudo o que
eles instanciam; em *Simulação*, cada testbench com o que ele instancia. Os
módulos da biblioteca SAPHO aparecem apagados, marcados SAPHO e fechados;
instâncias dentro de blocos `generate` levam o caminho do bloco no nome
(`op_add.my_add`). Clicar abre a definição do módulo; o menu leva também à
linha da instância e copia o caminho hierárquico. A hierarquia atualiza
sozinha depois de cada Verilog (F7), simulação, build ou síntese que passa,
e o botão do título elabora na hora. Quando um `.v` muda depois da última
elaboração, o aviso no topo diz que ela está desatualizada. Uma elaboração
que falha (um testbench com erro de sintaxe, por exemplo) aparece com o erro
e a linha, e não esconde as outras. Ela precisa do Icarus instalado,
qualquer que seja o simulador escolhido. É a mesma árvore do
`aurora hierarchy`.

**Arquivos** mostra a pasta do projeto: criar, renomear (F2), mandar para a
lixeira (Delete), mostrar os ocultos. Renomear e mover são o `aurora move`:
um `.v` registrado continua no projeto com o nome novo. O topo e o testbench simulado têm a
marca deles, e um ponto azul marca os outros arquivos registrados no `.spf`.
O arquivo da aba ativa aparece na árvore, com as pastas dele abertas, e as
pastas que você abre ficam lembradas por projeto.

**Arrastar e soltar:**

| De | Para | O que acontece |
|---|---|---|
| arquivo ou pasta da árvore de arquivos | uma pasta (ou o espaço vazio, para a raiz) | move; os `.v` registrados continuam registrados, com o topo e o testbench escolhido acompanhando |
| um `.v` das Fontes | Módulos ou Testbenches | muda o papel do arquivo |
| arquivos do gerenciador de arquivos do sistema | uma pasta da árvore de arquivos | copia para lá |
| `.v`, `.sv` ou `.py` do sistema | Módulos ou Testbenches | registra no lugar, sem copiar: em Testbenches, como testbench; em Módulos, com o papel que o conteúdo indica |
| um `.spf` do sistema | qualquer lugar | abre o projeto |
| outros arquivos do sistema | o editor | abrem em abas |

Com um arquivo do mesmo nome no destino, o Studio pergunta antes de
substituir, e o que estava lá vai para a lixeira. O `.spf`, a pasta `.aurora8`
e as pastas dos processadores não se movem: a AURORA os acha pelo lugar.

**Se o `.spf` for apagado** com o projeto aberto (pelo gerenciador de
arquivos, pelo terminal, por um `git`), o Studio avisa que sem ele não há
operação de projeto: compilar, verificar, simular, sintetizar, a placa,
os sinais da onda, o histórico e mexer nas fontes ficam desabilitados, e
o nome do projeto na barra de status ganha um alerta. A pasta continua
aberta: as abas, salvar, a árvore de arquivos, a busca, renomear e mover
(só no disco, já que não há `.spf` para manter em dia) continuam. No alto
do Explorador aparece o aviso com o botão **Criar um novo projeto**, que
abre o Novo projeto com esta pasta e este nome: criar ali grava só um
`.spf` novo, sem apagar arquivo nenhum, e as fontes e os processadores
precisam ser registrados de novo. As Fontes, a Hierarquia e o Navegador
de fluxo, que vêm do `.spf`, mostram só o aviso. Se o `.spf` voltar
(restaurado da lixeira ou do git), o projeto volta sozinho.

Nas três vistas, as setas andam pela árvore: cima e baixo de linha em
linha, direita abre (ou entra), esquerda fecha (ou sobe para o pai), Home e
End vão ao começo e ao fim, Enter abre e Ctrl+Enter abre no grupo do editor
ao lado.

O último botão da linha do título recolhe a árvore inteira e, clicado de
novo, expande tudo. Nas Fontes e na Hierarquia vale para as seções e os
nós; na árvore de arquivos, para as pastas (o "expandir" abre até 400,
contando só as ocultas que estiverem à mostra). Para esconder a barra
lateral inteira, clique de novo no ícone da vista aberta ou use Ctrl+B.

### 2.2 Editor dividido

O editor se divide em até três grupos lado a lado, como os painéis da
AURORA. Cada grupo tem as suas abas e o seu editor; o grupo com o foco tem
a aba ativa marcada em azul, e é dele que falam os atalhos (Ctrl+S, Ctrl+W,
localizar) e a barra de status.

| Para | Faça |
|---|---|
| abrir a aba ativa também à direita | `Ctrl+\`, o botão de colunas no fim da barra de abas, ou "Dividir à direita" no menu da aba |
| abrir um arquivo do explorador ao lado | "Abrir ao lado" no menu do arquivo, ou Ctrl+Enter na árvore |
| mudar uma aba de grupo | arraste a aba para a barra de abas de outro grupo (na posição) ou para o conteúdo dele (no fim); ou "Mover para o grupo da direita/esquerda" no menu da aba |
| criar um grupo arrastando | solte a aba na faixa da direita do conteúdo de um grupo (enquanto houver menos de três) |
| ir para o grupo 1, 2 ou 3 | Ctrl+1, Ctrl+2, Ctrl+3, ou clique nele |
| fechar um grupo | o X no fim da barra de abas dele, ou Exibir > Fechar o grupo do editor |

O mesmo arquivo pode estar em dois grupos: é o mesmo texto nos dois, o que
se digita num aparece no outro, e cada um guarda o seu cursor e a sua
rolagem. Fechar a aba num grupo não pergunta nada enquanto o arquivo estiver
aberto em outro; a pergunta de salvar vem quando ele sai do último. Uma
vista (preferências, esquemático, relatórios) fica num grupo só: dividir a
leva para o lado. Um grupo que fica sem abas desaparece, e os grupos
abertos voltam com o projeto.

### 2.3 Modo zen

Ctrl+K Z (aperte Ctrl+K, solte, aperte Z), Exibir > Modo zen ou a paleta.
Some tudo menos o editor: barras de menus, de ferramentas, de atividades e
de status, a barra lateral, o painel inferior e as abas. A janela entra em
tela cheia e, com um grupo só, o editor fica numa coluna de umas 110
colunas no meio da tela, sem o minimapa. Sem a tela cheia (Preferências >
Editor > Modo zen), a barra de título fica, sem os menus, para mover e
fechar a janela.

| Para | Faça |
|---|---|
| sair | Ctrl+K Z de novo, ou Esc duas vezes seguidas |
| usar o terminal | Ctrl+` abre o shell numa gaveta embaixo do editor, o mesmo do painel (o `aurora` está no `PATH`); Ctrl+` de novo fecha e o foco volta ao editor. A divisa entre os dois se arrasta |
| trocar de arquivo | Ctrl+P, ou a paleta (Ctrl+Shift+P) |
| rodar o fluxo | F5 a F10, como sempre |
| ver o painel ou uma barra lateral | Ctrl+J, Ctrl+B, Ctrl+Alt+B; valem só enquanto o zen durar |

Durante uma operação, um indicador pequeno no canto inferior direito mostra
a etapa e o tempo; ao terminar, mostra o resultado e os erros e avisos por
alguns segundos, e clicar nele abre Problemas. Os marcadores de erro
continuam no editor. Com a gaveta aberta o indicador sobe para o canto de
cima.

O Esc duas vezes não sai do zen com o foco no terminal (o Esc é do shell,
de programas como `vim` e `less`) nem no editor com o modo Vim ligado (o Esc
é do Vim); nesses casos, Ctrl+K Z. Ao sair, as barras laterais, o painel e
a janela voltam como estavam; se a janela já estava em tela cheia antes,
continua. O Studio sempre abre fora do zen.

Preferências > Editor, no grupo Modo zen: entrar em tela cheia,
centralizar o editor, mostrar as abas, esconder os números de linha.

### 2.4 Layout da janela

Há três regiões: a barra lateral esquerda, a direita e o painel, que fica
embaixo ou à direita do editor. As quinze vistas (Explorador, Fluxo, Busca,
Relatórios, Exercícios, Assistente, o INFO, os terminais de etapa C±, ASM,
Verilog, Wave, RTLViewer e Placa, Problemas e Terminal) vão para qualquer
uma delas. A Placa e o Assistente só existem com o recurso experimental
ligado (seção 9).
O editor fica sempre no meio.

| Para | Faça |
|---|---|
| mover uma vista | botão direito na aba ou no ícone dela > Mover para |
| esconder uma vista | botão direito > Esconder; ela volta pelo menu Exibir (Exibir > Consoles, para um terminal, ou pelo seletor de terminal da seção 6), pela paleta ou por Preferências > Layout da janela |
| esconder uma barra ou uma região | Exibir > Aparência, ou o botão direito em qualquer barra |
| esconder um botão da barra de ferramentas ou um item da barra de status | botão direito na barra: a lista marca o que aparece |
| trocar o modelo da barra de ferramentas | Preferências > Layout da janela, ou o botão direito na barra > Modelo da barra |
| pôr o painel à direita do editor | Exibir > Aparência > Painel à direita |
| pôr a barra de atividades à direita, ou escondê-la | Exibir > Aparência |

A barra de atividades escolhe a vista da barra lateral do lado dela. A
outra barra lateral, ou as duas com a barra de atividades escondida,
mostram as vistas em abas com ícone no cabeçalho.

Esconder a barra de menus tira só os menus: a barra de título continua,
com o título e os botões da janela.

O que foi escondido sempre tem volta: a paleta (Ctrl+Shift+P) e o botão
direito na barra de abas do editor, que aparece sempre que há uma aba
aberta, mostram todas as barras e regiões. Sem a barra de status, o
indicador do canto, o mesmo do zen, mostra a operação, o resultado e a
linha do Vim. Sem o seletor de alvo na barra de ferramentas, o alvo se
escolhe pela paleta (Ctrl+Shift+P), em Escolher o alvo.

As Preferências e o RTLViewer usam a janela inteira: enquanto um deles está
na frente, as barras laterais e o painel ficam escondidos, e voltam como
estavam quando outra aba vem para a frente. Nesse meio tempo, Ctrl+B,
Ctrl+J e os botões da barra de status ainda os mostram. O layout salvo não
muda com isso.

Numa janela estreita demais para tudo, a barra lateral direita deixa de
aparecer e depois o painel volta para baixo do editor. O layout continua o
mesmo: com a janela larga de novo, tudo volta.

**Modelos da barra de ferramentas.** Cada layout escolhe onde ficam os
botões do fluxo. Preferências > Layout da janela mostra um desenho de cada
modelo; o botão direito na barra também troca, em Modelo da barra.

| Modelo | Onde | Como funciona |
|---|---|---|
| Clássica | numa linha abaixo dos menus | os botões de arquivo, o Alvo e os do fluxo lado a lado, como na seção 5 |
| Na barra de título | no fim da barra de título, antes dos botões da janela | o Alvo, o botão **Executar** e o **Parar**. O Executar roda a etapa que mostra (de início, Simular). A seta ao lado dele abre o menu com todas as etapas e os atalhos, e a etapa escolhida ali passa a ser a do botão. Abrir a onda, escolher os sinais e mostrar a placa rodam pelo menu sem mudar o botão |
| No editor | no canto das abas do editor em uso; sem nenhuma aba aberta, some com a barra de abas | os mesmos controles do modelo anterior, com o mesmo comportamento. Com o fonte C± ou C++ de um processador na frente, o Executar sugere **Compilar** esse processador; com qualquer outro arquivo, fica a etapa escolhida no menu |

Nos dois últimos, a linha da barra de ferramentas some e o editor ganha a
altura dela. Sem a barra de título integrada (em tela cheia), o Executar
do segundo modelo fica no fim da barra de menus. O botão direito na barra
esconde os itens do modelo em uso.

**Layouts com nome.** Um layout é uma foto da janela. Mexer na janela
(Ctrl+B, Ctrl+J, arrastar um divisor, mover uma vista) não muda a foto.
Preferências > Layout da janela mostra o layout em uso e avisa quando a
janela está diferente da foto.

| Para | Faça |
|---|---|
| gravar a janela no layout em uso | Exibir > Layout > Salvar layout |
| criar outro layout | Exibir > Layout > Salvar layout como |
| voltar à foto | Exibir > Layout > Restaurar layout |
| trocar de layout | Ctrl+K L, ou Exibir > Layout > Selecionar layout |
| voltar ao Padrão | Exibir > Layout > Voltar ao layout padrão |
| renomear, excluir, ajustar tudo numa tela | Preferências > Layout da janela |

Há três layouts prontos: o Padrão, a janela de antes dos layouts, e dois
com outro modelo da barra de ferramentas, **Executar na barra de título** e
**Executar no editor**. O último também põe o Navegador de fluxo na barra
lateral direita, com todas as etapas à vista. Os prontos não mudam: salvar
sobre um deles pede um nome e cria outro. Trocar de layout descarta o que
não foi gravado no anterior. Os layouts ficam nas preferências (`settings.json`, seção 9),
valem em qualquer projeto e são os mesmos no Windows, no Linux e no macOS;
a janela de agora, com os tamanhos, é lembrada ao fechar o Studio.

## 3. Projeto Verilog

1. **Arquivo > Novo projeto** (Ctrl+Alt+N): nome e pasta. O Studio cria
   `<pasta>/<nome>/<nome>.spf` e abre o projeto. A pasta começa na última
   que você usou ou, na primeira vez, em `Projetos` dentro da sua pasta de
   usuário (`~/Projetos` no Linux e no macOS, `C:\Users\<nome>\Projetos` no
   Windows), criada se falta; Abrir projeto começa nela também. O nome usa
   só letras sem acento, números, `_` e `-`, começando por letra: ele vira
   pasta, `.spf` e parte do caminho que as ferramentas recebem. A regra é a
   da AURORA (`aurora new` recusa igual), e o diálogo avisa enquanto você digita.
2. **Projeto > Novo módulo Verilog**: dê o nome do arquivo (`contador.v`).
   A AURORA cria o módulo a partir do modelo; o primeiro módulo vira o topo.
3. Escreva o módulo e salve (Ctrl+S).
4. **Projeto > Novo testbench**: o nome sugerido é `<topo>_tb.v`. Um
   testbench `X_tb.v` ou `tb_X.v` já sai instanciando o módulo `X` com todas
   as portas, com clock e reset se ele tiver `clk` e `rst`, gravando a onda
   e terminando com `$finish`.
5. **Verilog** (F7): verifica o design e cada testbench com o Icarus. Os
   erros aparecem no INFO, no painel Problemas e sublinhados no editor.
6. **Wave** (F8): simula e abre a onda no surfer-aurora, numa aba (seção
   4.1). O que o testbench escreve (`$display`) aparece no INFO enquanto
   roda, embaixo de "saída do testbench".
7. **RTLViewer** (F10): sintetiza o topo com o Yosys e abre o esquemático
   (seção 7).

Arquivos que já existem entram por **Projeto > Adicionar arquivos Verilog
ou cocotb** ou arrastando-os para Módulos ou Testbenches, no explorador
(seção 2.1). O
AURORA decide pelo conteúdo se cada um é módulo ou testbench (a regra da
AURORA); soltar em Testbenches marca o arquivo como testbench. Um arquivo de
fora da pasta do projeto é registrado no lugar, sem cópia: no mesmo
repositório git do projeto, o `.spf` guarda o caminho relativo
(`../../rtl/x.v`), que vale em qualquer clone; fora de repositório, o
absoluto.

**Testbench em Python (cocotb).** **Projeto > Novo testbench cocotb
(Python)**, ou **Novo testbench** com a linguagem Python, cria um `.py` com
os testes do cocotb; o nome sugerido é `test_<topo>.py`, e o `test_X.py`
sai com a linha `# aurora-toplevel: X`, que diz qual módulo os testes
recebem como `dut` (a diretiva da AURORA). Sem essa linha, os testes recebem
o módulo de topo, com um aviso. O nome do arquivo vira o nome do módulo
Python: só letras, números e `_`, sem começar por número. Um `.py` que já
existe entra como os outros arquivos, sempre em Testbenches; o explorador
lista na seção dos não registrados os `.py` da pasta que têm
`@cocotb.test`. Com o `.py` como testbench simulado, **Wave** (F8) e
**Rápida** (F9) rodam os testes no simulador das preferências, o Icarus ou o
Verilator: o log do cocotb sai no INFO, cada teste aparece lá com o
resultado, e o que falhou vai também para o painel Problemas, na linha do
`.py`. Com a Wave, a onda abre mesmo com teste falhando, porque é nela que
se vê a falha (`.fst` no Icarus, `.vcd` no Verilator); a Rápida roda os
testes sem gravar onda. O cocotb precisa do componente cocotb
(**Ferramentas**), e o **Verilog** (F7) verifica o design sem o `.py`. A
primeira simulação cocotb da máquina leva alguns segundos a mais, enquanto o
Python compila o cocotb; no Verilator, a primeira de cada testbench compila
também o modelo.

**A ordem dos arquivos** em Módulos e em Testbenches é a ordem em que os
compiladores os leem, e um `` `define `` só vale para os arquivos de baixo
dele (a verificação acusa a macro indefinida). Para mudar, arraste o
arquivo sobre outro da mesma seção (ele entra antes desse), ou use **Subir
na lista** e **Descer na lista** no menu do arquivo.

**O topo** pode ser qualquer arquivo Verilog, inclusive o `.v` que o build
de um processador gerou (`<proc>/<proc>/<proc>.v`) e um `.v` que ainda não
está no projeto, que passa a ser registrado. A exceção é o nome de
testbench: `tb_<nome>.v`, `<nome>_tb.v` e `tb.v` não viram topo. Escolha
pelo botão direito no arquivo (nas Fontes, na árvore de arquivos ou na
hierarquia) ou por **Projeto > Escolher o módulo de topo**, que lista todos os
candidatos e diz de onde cada um vem. O `.v` gerado escolhido como topo
continua sendo refeito a cada build; registrado, ele aparece uma vez só em
Módulos, com as marcas Gerado e Topo. Para trocar o topo ou o testbench
simulado, clique com o botão direito no arquivo, no explorador.

## 4. Processador SAPHO

1. Crie ou abra um projeto.
2. **Projeto > Novo processador SAPHO** (Ctrl+Alt+P): nome (letras,
   números e `_`, sem começar por número), linguagem (C± ou C), portas de
   entrada e de saída e, em C±, a arquitetura (largura, mantissa, expoente,
   ganho, pilhas; as pilhas vazias ficam por conta do YANC, que as calcula
   pelo programa). Os padrões são os da AURORA. O Studio cria a pasta
   `<processador>/` com o fonte `<processador>.cmm` (ou `.cpp`) e o abre.
   Tudo o que o YANC gera vai para `<processador>/<processador>/`, a pasta
   de saída, que nasce no primeiro build: a estrutura do YANC 7.1, a mesma
   da AURORA.
3. Escreva o programa. O processador se chama como o arquivo, e o fonte não
   leva `#PRNAME`: o YANC 7.1 recusa a diretiva e diz para apagar a linha.
4. Entradas: no explorador, botão direito no processador > **Novo arquivo
   de entrada**. É o `<processador>/input_<n>.txt`, ao lado do fonte, da
   porta `n` de `in(n)`, um valor por linha. As saídas da simulação ficam
   na pasta de saída (`output_<n>.txt`).
5. Escolha o processador no **Alvo** da barra de ferramentas. Com o alvo
   num processador, os botões agem só nele; com o alvo em "Projeto", agem no
   projeto inteiro (todos os processadores e o testbench do projeto).
6. **C±** (F6): compila (cmmcomp ou cppcomp, depois asmcomp). Os erros do
   YANC apontam a linha do fonte, ou do cabeçalho incluído, em C.
7. **Wave** (F8) ou **Rápida** (F9): simula o processador com o testbench
   que o YANC gerou; a Rápida, no Verilator e sem gravar onda. No fim, o
   INFO mostra os valores de cada porta de saída (`Saída 0: 55`), e
   a aba do processador também.

A **aba do processador** (duplo clique nele no explorador) tem a frequência,
o número de clocks, a exportação de arrays para a onda e o tempo simulado
que eles dão, as entradas e as saídas da última simulação. A frequência e os
clocks valem a partir do próximo build.

Para tirar um processador do projeto: botão direito nele no explorador >
**Remover processador...**, ou Projeto > Remover processador, que pergunta
qual. A confirmação oferece duas saídas:
- **Remover do projeto**: sai a entrada no `.spf`, e com ela o Verilog e o
  testbench gerados nas listas de arquivos (se um deles era o topo ou o
  testbench simulado, a escolha fica vazia). A pasta do processador fica no
  disco, e os `.v` gerados nela aparecem em Fora do projeto.
- **Remover e mandar a pasta para a lixeira**: o mesmo, e a pasta inteira
  (fonte, entradas, arquivos gerados) vai para a lixeira do sistema, de
  onde dá para recuperar. As abas dos arquivos dela fecham.

A aba do processador fecha nos dois casos. Durante uma operação, a remoção
fica desabilitada. Na CLI, é o `aurora proc remove`.

### 4.1 A onda

Wave (F8), ou Onda (Ctrl+F8) depois de simular, abre a onda no Surfer. A
onda de um processador abre arrumada como na AURORA: um grupo por
processador com o `clk` e o `rst`, as portas (I/O), as **instruções** e as
**variáveis** do programa, e as flags (pilhas e ULA, fechadas).

- **Assembly**: a instrução que o processador executa em cada ciclo
  (`LOD_V main_x 2`, `SET main_soma`), no lugar do número do PC.
- **C±**: a linha do fonte daquela instrução, com o texto da linha.
- **Variáveis**: `int` com sinal, `float` como número real, complexo como
  `re imi`; cada array num grupo fechado (só com a exportação de arrays
  ligada, na aba do processador).

O layout sai das tabelas que o YANC deixa ao compilar; sem elas (um
processador compilado fora da AURORA), o PC e a linha aparecem como números.
Também aparecem como números quando o processador foi compilado de novo
depois da simulação, porque as tabelas novas deslocariam o assembly e a
linha: o resumo da aba avisa "compilado depois desta simulação", e basta
simular de novo. A onda de um projeto só de Verilog abre com o grupo
Top-level, com os sinais do testbench; o resto do design fica na
hierarquia do Surfer.

Onde a onda abre é uma preferência (Preferências > Simulação): **numa
aba** do Studio (o padrão) ou **em janela separada**. Na aba, o Surfer roda
dentro do Studio, com os menus e os atalhos dele; com o foco na aba, letras,
setas, espaço e as teclas de edição são do Surfer, e as teclas de função
(F5 a F10) e as combinações com Ctrl, Alt ou Cmd continuam sendo do Studio
(Ctrl+W fecha a aba, Ctrl+B esconde a barra lateral); a F11, o Ctrl+K e o
Ctrl+S ficam com o Surfer. No alto da aba ficam o arquivo, o resumo de cada
processador, onde fica o layout salvo, **Voltar ao layout gerado** (quando
você salvou um), o botão **Sinais** (na onda do projeto, com a escolha:
"Sinais: todos" ou "Sinais: 3 marcados"), **Ler a onda de novo** e **Abrir
em janela**. Trocar de aba não perde
nada: a aba de onda guarda o zoom, o cursor e os sinais acrescentados
enquanto estiver aberta (cada aba de onda aberta ocupa memória; feche as
que não usa). Simular de novo recarrega a aba aberta da onda. A aba precisa
do cliente web do Surfer no bundle: sem ele, a onda abre em janela. Uma onda
de mais de 256 MB não abre na aba, que avisa e oferece **Abrir em janela**.

**O layout salvo.** A onda de um testbench do projeto abre com o layout de
`wave/<testbench>.surf.ron`, na pasta do projeto, e o **Ctrl+S** do Surfer
salva nele: na aba e na janela. Enquanto você não salva, o Studio refaz o
arquivo a cada abertura com o layout gerado (a escolha de sinais ou os
processadores podem ter mudado); depois que você salva, a onda abre sempre
com o seu, e **Voltar ao layout gerado**, no alto da aba, o apaga. O arquivo
pode ir para o git com o projeto.

**Escolher os sinais da onda** é a Wave Configuration da AURORA. Ela abre
pelo botão **Sinais** da barra de ferramentas, ao lado de Onda (o número no
botão é quantos itens a escolha tem; sem número, a onda grava todos), pelo
botão Sinais no alto da aba de onda, pelo Navegador de fluxo (Simulação),
pelo menu Fluxo e pelo menu de contexto do testbench simulado no Explorer.
A janela mostra a árvore do testbench do projeto, elaborada pelo Icarus, com
uma caixa por escopo e por sinal. Marcar um escopo vale por tudo o que há dentro dele; desmarcar um
sinal dentro de um escopo marcado deixa os outros. O filtro procura pelo
nome, com texto ou expressão regular; **Só o testbench** marca os sinais do
módulo do testbench, e **Desmarcar tudo** volta a gravar todos. **Gravar a
escolha** escreve `wave/<testbench>.json`: a próxima simulação grava só os
marcados, e o layout mostra só eles, um grupo por escopo; **Gravar e
simular** grava e roda a Wave (F8) do projeto, qualquer que seja o alvo, e a
onda abre com a escolha nova. Um processador
SAPHO entra sempre, porque as instruções e as variáveis dependem dele. Nada
marcado é o padrão: a onda grava todos os sinais. No Verilator, que grava
tudo de qualquer jeito, o layout mostra só os marcados.

**Projeto da estrutura antiga.** Até o YANC 6.x cada processador tinha
`Software/`, `Hardware/` e `Simulation/`, que o YANC 7.1 não compila. Ao
abrir um projeto assim, o Studio pergunta se pode converter, como a AURORA:
o fonte e os arquivos da pessoa vão para a pasta do processador, o que o
YANC gerou vai para a pasta de saída, nada é apagado nem sobrescrito, a
linha `#PRNAME` sai do fonte e o `.spf` passa a apontar para os lugares
novos. Depois, uma janela diz o que mudou de lugar, o que ficou e por quê,
e avisa quando o módulo Verilog de um processador mudou de nome. Recusada,
a conversão fica em **Arquivo > Converter projeto para o YANC 7.1**, e o
painel de problemas avisa até lá (`aurora convert` faz o mesmo no terminal).

## 5. Os botões do fluxo

Os botões são os da barra de ferramentas clássica. Nos outros dois modelos
(seção 2.4), as mesmas etapas ficam no menu da seta do **Executar**, com as
mesmas teclas.

| Botão | Tecla | Faz | Comando equivalente da AURORA |
|---|---|---|---|
| C± | F6 | compila os processadores (o alvo, ou todos) | `aurora build [-p NOME]` |
| Verilog | F7 | verifica o Verilog com o Icarus, sem compilar os processadores (o Verilog do YANC entra como está no disco; compile antes com F6); com um processador no alvo, verifica o Verilog dele com o testbench do YANC | `aurora check [-p NOME]` |
| (menu Fluxo) | Shift+F7 | o mesmo, com o lint do Verilator | `aurora check [-p NOME] --lint` |
| Wave | F8 | compila, simula e abre a onda | `aurora sim [-p NOME] --open` |
| Rápida | F9 | a simulação rápida, o Fast Sim da AURORA: compila e simula sem gravar onda, para ver a saída, as portas e os testes. O testbench Verilog e o de um processador rodam no Verilator, qualquer que seja o simulador escolhido; um testbench cocotb, no simulador escolhido. O navegador de fluxo mostra qual | `aurora sim [-p NOME] --fast` |
| Onda | Ctrl+F8 | abre a onda da última simulação | `aurora wave [-p NOME]` |
| Sinais | | escolhe os sinais que a onda do projeto grava e mostra (seção 4.1); o número no botão é quantos itens a escolha tem | `aurora wave signals`, `aurora wave select` |
| RTLViewer | F10 | compila, sintetiza e abre o esquemático (seção 7) | `aurora synth [-p NOME]`; o `--svg` desenha com o Graphviz |
| (menu Fluxo) | F5 | compila os processadores, verifica e, se passar, simula | |
| Parar | Shift+F5 | cancela a operação; a ferramenta é encerrada com tudo o que iniciou. A atualização da AURORA não para no meio | Ctrl+C na CLI |

Antes de cada operação, o Studio salva os arquivos abertos: compila o que
está na tela. Uma operação por vez; os botões ficam desabilitados enquanto
uma roda. O simulador (Icarus ou Verilator) é escolhido no menu Fluxo ou
em Preferências > Simulação.

Um botão que não pode fazer nada fica desabilitado, em vez de dar erro:
Fluxo > Verilator e o lint (Shift+F7) sem o Verilator instalado, Onda antes
de existir a onda do alvo.

Um testbench sem `$finish` não termina sozinho: use Parar, ou defina um
prazo em Preferências > Simulação.

Cada operação grava um relatório no histórico do projeto (`.aurora8/reports/`),
como a CLI. O número aparece no fim da rodada, no INFO.

## 6. INFO, terminais, problemas e shell

O painel abre com dois terminais lado a lado: o **INFO**, com o que as
operações escrevem, e o **Terminal**, o shell. Depois deles vem
**Problemas**.

**INFO.** Cada operação é uma rodada, do mesmo jeito em todos os fluxos:

```
── Simulação · 14:02:31 ───────────────────────
$ aurora-ide sim -p soma
▸ Compilando processadores (YANC)
  cmmcomp
    (o que o cmmcomp escreveu)
✓ Build de soma (C±, 100 MHz, 2000 clocks): terminou em 155 ms
▸ Simulando
  saída do testbench
    (o $display do testbench)
✓ Simulação de soma_tb (Icarus): terminou em 524 ms
    Saída 0: 55
    Relatório run-000042 gravado no histórico
── ✓ Terminou em 0,7 s · erros: 0, avisos: 0 ──
```

- A rodada começa com a operação e a hora, e termina com o status, o tempo
  e quantos erros e avisos ficaram em Problemas.
- Cada fase começa com ▸, em negrito. Embaixo vem o nome de cada
  ferramenta que escreveu algo, em cinza, e, recuado, o que ela escreveu,
  sem mudança. Na simulação, o que o testbench escreve fica embaixo de
  "saída do testbench".
- A cor só mostra como algo terminou: ✓ verde, ✗ vermelho, ! amarelo, e a
  palavra de gravidade nas mensagens das ferramentas (`error`, `warning`).
  Em cinza fica o secundário: o comando, o nome da ferramenta, onde ficou a
  onda, o relatório.
- O INFO vem para a frente quando a operação começa e não troca de aba
  no meio dela. Quem está no Terminal ou num terminal de etapa fica onde
  está, e a aba do INFO ganha um ponto. Numa falha, ele rola até o primeiro
  erro da rodada.
- Um `arquivo:linha` é um link, também com espaço ou acento no caminho:
  clique para abrir o arquivo na linha. Com a preferência "Mostrar o
  comando de cada passo e as mensagens info nos consoles" (Preferências >
  Simulação), aparece também a linha de comando exata de cada ferramenta,
  para repetir à mão.

**Terminais de etapa.** O botão de escolher o terminal (as setas para cima
e para baixo, no cabeçalho de qualquer terminal) lista o INFO, o Terminal e
um terminal por etapa, que correspondem aos terminais da AURORA:

| Terminal | Na AURORA | O que mostra |
|---|---|---|
| C± | TCMM | o compilador de C± ou de C (cmmcomp, cppcomp) |
| ASM | TASM | o montador (asmcomp) e o resumo do build |
| Verilog | TVERI | a verificação do Icarus e o lint do Verilator |
| Wave | TWAVE | a elaboração e a simulação: o que o testbench escreve, as saídas do processador, onde ficou a onda |
| RTLViewer | TRTLViewer | a síntese do Yosys |
| Placa | | a compilação e a gravação pelo Quartus; só existe com a placa FPGA ligada (seção 7.2) |

Os abertos têm a marca. Escolher um fechado abre ele com o que a etapa já
escreveu, separado por rodada, e daí em diante ele recebe as linhas novas
da etapa, que continuam indo também para o INFO. O terminal fica aberto,
inclusive nas próximas vezes que você abrir o Studio, até ser fechado pelo X
do cabeçalho; Exibir > Consoles também abre qualquer um.

**Problemas** lista os erros e avisos da última operação, com filtro;
clicar abre o arquivo na linha. As mensagens `info` aparecem marcando a
caixa "info". No topo ficam os avisos do `.spf` do projeto aberto (ferramenta
`.spf`), que valem enquanto ele está aberto: um caminho de outra máquina
(`C:\...`) achado dentro da pasta do projeto, que a AURORA grava como o de
hoje na próxima mudança; um topo ou testbench escolhido fora da lista, que é
ignorado; um processador com nome que não compila.

**Terminal** é um shell de verdade, aberto na pasta do projeto, com o
`aurora` no `PATH`: `aurora status`, `aurora report compare`, tudo o que a CLI
faz. Ao abrir outro projeto, o terminal vai junto: o Studio digita no shell
o `cd` para a pasta nova (`Set-Location` no PowerShell, `cd /d` no Prompt
de Comando), e o histórico e o que estava na tela ficam. No Linux e no
macOS, se um programa estiver rodando no terminal nessa hora, o Studio não
digita o `cd` e avisa no terminal que ele ficou na pasta anterior. No
Windows, onde o terminal não informa se há um programa rodando, o `cd` vai
para o programa como entrada: encerre-o antes de trocar de projeto. O botão
`+` abre outro no lugar do atual. No Linux e no macOS é o seu
shell (a variável `SHELL`). No Windows, Preferências > Geral escolhe
entre o PowerShell (o padrão) e o Prompt de Comando (`cmd`); trocar
reinicia o terminal aberto com o shell novo.

## 7. Esquemático, síntese e relatórios

- **RTLViewer** (o esquemático): o Studio desenha o netlist da síntese
  (`hierarchy.json`) com as cores do tema, sem Graphviz. Cada família de
  célula tem um tom da paleta do tema (aritmética, lógica, comparação,
  multiplexador, registrador, memória, submódulo), listado no rodapé;
  passar o mouse numa família da legenda acende as células dela. O uso
  segue o RTLViewer da AURORA:

  | Gesto | O que faz |
  |---|---|
  | Roda do mouse, `+`, `-` | zoom no ponto do mouse; `0` volta ao tamanho real, `F` ajusta à janela |
  | Arrastar, setas | move o desenho |
  | Passar o mouse num fio | acende a rede, com as fatias dela, e mostra nome, largura e destinos |
  | Clique | seleciona a célula ou a rede: as ligações acendem e os detalhes aparecem à direita |
  | Dois cliques | numa caixa de submódulo, entra nele; num símbolo ou fio, abre o código na linha |
  | Ícone ↗ da caixa, Enter | entra no submódulo |
  | Esc, Alt+←, botão lateral do mouse | tira a seleção; sem seleção, volta |
  | Alt+→, o outro botão lateral | avança |
  | Ctrl+F | busca célula, rede ou porta no módulo; Enter vai ao próximo, Shift+Enter ao anterior |
  | Botão direito | abrir o módulo, ir ao código, destacar, copiar o nome, exportar |

  A árvore à esquerda é a hierarquia de instâncias desde o topo, e a trilha
  no alto mostra o caminho até o módulo desenhado. Os detalhes mostram
  tipo, parâmetros, o código de origem e, por porta, a rede ligada; sem
  seleção, o resumo do módulo. Na barra: nomes das redes nos fios, largura
  dos barramentos, redes globais como rótulo (uma entrada do módulo com 8
  destinos ou mais, como `clk` e `rst`, aparece escrita em cada destino em
  vez de roteada), os dois painéis, exportar SVG (com as cores do tema,
  pronto para documento) e sintetizar de novo. Constantes aparecem escritas
  na porta que recebem. O layout é do ELK e roda fora da interface; um
  módulo com mais de 1500 células pede confirmação antes. Depois de reabrir
  o Studio, "Mostrar a última síntese" abre o netlist que ficou no disco.
- **Síntese** (Fluxo > Estatísticas da síntese): células, fios, memórias e
  processos do `stat` do Yosys, e as células por tipo. São células
  genéricas, sem mapeamento para FPGA: não há LUT, DSP nem temporização.
  "não informado" é diferente de zero.
- **Relatórios** (barra de atividades): o histórico do projeto, do mais novo
  para o mais antigo. Abrir mostra o relatório (máquina, ferramentas e
  versões, tempo de cada passo, estatísticas); **Comparar** mostra a
  diferença de síntese e de tempos contra o anterior comparável, com os
  avisos de contexto (fontes, versões, máquina diferentes).
- **Limpar relatórios** (a lixeira no título dos Relatórios, ou Ferramentas
  > Limpar relatórios): o `aurora report clean`. Escolha manter os N mais
  novos ou apagar todos; o diálogo mostra quais saem antes de apagar. O
  menu de cada relatório (botão direito) apaga só ele. O número de um
  relatório apagado não volta a ser usado. Com uma operação rodando, a
  limpeza espera.

## 7.1 Exercícios

A vista **Exercícios** (Exibir > Exercícios, ou Ferramentas > Exercícios)
é o `aurora learn` no Studio: exercícios curtos de Verilog, corrigidos a cada
gravação contra uma solução de referência
([docs/LEARN.md](../../docs/LEARN.md) da AURORA). Eles vêm no componente
`aurora-learn`; sem ele, a vista oferece instalá-lo.

- **Criar a pasta de exercícios** escolhe onde, e cria lá a pasta
  `aurora-learn`, com um projeto por exercício. **Abrir uma pasta de
  exercícios** abre uma que já existe. A pasta fica lembrada.
- A lista mostra os capítulos, os exercícios resolvidos (com a marca verde)
  e o atual. Clicar num exercício abre o projeto dele, o seu arquivo à
  esquerda e o enunciado à direita, sem corrigir.
- Com **Corrigir ao gravar** ligado (o padrão), cada gravação do arquivo do
  exercício corrige, também quando o arquivo é gravado por outro editor.
  Ctrl+Alt+L e o botão **Corrigir** corrigem quando quiser.
- Com um exercício aberto, o Studio fica numa sessão de exercícios: trocar
  de aba não grava sozinho (mesmo com **Salvar ao trocar de aba ou de
  janela** ligado), e
  o painel de baixo fica escondido até você chamá-lo (Ctrl+J). A correção
  não traz o INFO para a frente: o resultado aparece na aba do enunciado. Abrir
  outro projeto ou fechar a pasta de exercícios encerra a sessão, e o painel
  volta como estava.
- Sair da vista Exercícios (o Explorador, a Busca ou outra vista na frente
  dela) e fechar a pasta de exercícios fecham o projeto do exercício, com as
  abas dele. Com o arquivo alterado e não gravado, o Studio pergunta antes;
  **Cancelar** mantém o exercício e volta à vista Exercícios. Recolher a
  barra lateral não fecha nada.
- A aba do enunciado mostra a correção: resolvido, não compila, ou em
  quantas amostras cada saída errou e o instante do primeiro erro. Os
  erros de compilação levam à linha e também vão para Problemas e para o
  editor. Quando dá, a correção diz a causa: uma porta com outro nome, uma
  saída sem atribuição, um reset que só erra quando ativo.
- Os botões: **Dica** mostra a próxima dica; **Onda** abre a onda da última
  correção com as entradas, cada saída sua ao lado da de referência e o
  primeiro erro marcado; **Esquemático** sintetiza o seu circuito e o abre
  no RTLViewer; **Restaurar** volta o arquivo ao começo (pergunta antes);
  depois de resolvido, **Solução** abre a solução de referência e
  **Próximo** vai ao próximo exercício por resolver.

Sem o componente, a vista também deixa escolher a pasta das trilhas. Quem
escreve exercícios aponta Preferências > Bundle da AURORA > **Pasta das
trilhas de exercícios** para a pasta `aurora-learn` do repositório, e testa a
trilha sem instalar o componente. No `npm run tauri dev`, sem essa
preferência, o Studio já usa a pasta `aurora-learn` do repositório.

## 7.2 Placa FPGA

A placa FPGA é um recurso experimental e vem desligada. Ligue em
Preferências > Experimental > **Placa FPGA**, ou com `aurora experimental
enable fpga` num terminal: os dois mexem no mesmo arquivo, e o que um liga
vale para o outro (um comando dado num terminal fora do Studio vale quando
a janela volta a ter o foco). Desligada, a placa sai da janela: a aba Placa
FPGA, o botão **FPGA** da barra de ferramentas, as ações do menu Fluxo e da
paleta, a seção FPGA do Fluxo, o terminal **Placa** e o Quartus Prime das
Preferências e das Ferramentas da AURORA. Desligar com a aba aberta a fecha.
O `fpga.json` do projeto fica como está.

A aba **Placa FPGA** (o botão **FPGA** na barra de ferramentas, ou Fluxo >
Mostrar a placa FPGA) leva o projeto para a placa. No alto ficam a placa (a
DE2-115, a DE10-Nano ou a Cyclone V GX Starter Kit) e o topo que vai para
ela; vazio, o topo do projeto, ou o processador, se houver um só. Embaixo,
três etapas, cada uma com o estado:

1. **Ligações**: uma linha por porta do topo, com o sinal da placa, os bits
   dele (vazio: os de baixo) e **Inverter**. Os botões das placas são
   ativos em nível baixo: apertado lê 0, e a lista avisa ("ativo em 0").
   Marque **Inverter** quando a porta espera 1 com o botão apertado, como o
   `rst` do SAPHO. Uma entrada pode ficar fixa em 0 ou 1. Cada mudança é
   gravada no `fpga.json` e conferida na hora; um problema aparece na linha
   da porta. **Ligar automaticamente** liga as portas soltas: pelos nomes
   iguais aos da placa (um topo escrito para ela, com `CLOCK_50`, `KEY`,
   `SW`) e, num processador SAPHO, o `clk` no oscilador, o `rst` no
   primeiro botão, invertido, a entrada nas chaves e as saídas nos LEDs.
   Confira o que ele escolheu. **Como fica** mostra o resultado de cada
   ligação, e o Verilog do topo da placa gerado.
2. **Compilação**: o Quartus gera o arquivo de gravação. A etapa diz se ele
   está em dia com o projeto ou, se algo mudou depois, o quê.
3. **Gravação**: o arquivo vai para a placa pelo cabo dela. A etapa diz se
   o cabo está ligado; sem ele, ligue a placa pela porta USB BLASTER, com a
   chave RUN/PROG em RUN, e procure de novo. A placa roda o projeto até ser
   desligada; ao ligar de novo, ela carrega o que está na memória de
   configuração dela (na DE2-115, o projeto de fábrica).

**Compilar e gravar na placa** faz as duas últimas de uma vez. A saída do
Quartus vai para o INFO (e para o terminal **Placa**, se ele estiver
aberto), e os erros, para Problemas. No fim
aparecem os recursos da FPGA e, por clock, a frequência pedida, a Fmax e as
folgas. Sem o Quartus, os botões ficam desligados (Preferências > Bundle do
AURORA > Quartus Prime). Se o topo é um processador ainda não compilado, as
portas aparecem depois de **Compilar os processadores**.

## 7.3 Placa virtual

A placa virtual roda o projeto no computador com os periféricos da placa
simulados: dá para clicar nas chaves e nos botões, ver os LEDs e os displays,
digitar no teclado PS/2 e ver a tela VGA, antes de gravar a placa de verdade.
Ela vem com a placa FPGA (Preferências > Experimental > **Placa FPGA**) e
abre em Fluxo > Mostrar a placa virtual, ou pela paleta.

A aba desenha a placa do `fpga.json` do projeto. Sem `fpga.json`, ela pede a
placa e liga cada porta do topo que tem o nome de um sinal dela (`SW`, `KEY`,
`LEDR`, `HEX0`), sem gravar nada. Em cima ficam o monitor, a serial e o
teclado que o topo usa; os que ele não usa aparecem só numa linha. Embaixo
fica a placa: os displays, os LEDs sobre as chaves de mesmo bit e os botões,
como na DE2-115. Cada elemento mostra a porta do topo ligada a ele, em
faixas (`SW[17:3]`, `KEY[1]`, `LEDR`); o elemento sem ligação aparece
apagado.

**Editar ligações**, com a placa desligada, transforma cada chave, botão,
LED e display num alvo: um clique abre as portas do topo da direção certa,
bit a bit (ou em faixas da largura do display), com **Inverter** para um
botão ativo em 0. A escolha muda só aquele elemento, grava o `fpga.json` e
vale ao ligar a placa de novo; **não ligada** solta o elemento. Uma saída
que já ia a outro lugar continua lá, então dá para ver o mesmo valor nos
LEDs e num display. A tela, o teclado e a serial, que têm vários sinais, se
ligam na tabela da aba Placa FPGA (**Tabela de ligações**).

**Ligar a placa** compila o projeto pelo Verilator (a primeira vez leva de
15 a 30 segundos) e começa a simulação. Enquanto ela roda:

- as chaves mudam com um clique, e o botão fica apertado enquanto o mouse
  está em cima dele apertado; Shift+clique prende o botão apertado até o
  próximo Shift+clique, para segurar um botão enquanto se aperta outro;
- o brilho de cada LED é o tempo que ele ficou aceso, então um LED em PWM
  aparece mais fraco;
- com o teclado PS/2 ligado, clique na tela ou na área do teclado e digite:
  as teclas vão para o projeto, e só F1 a F12 e Ctrl+Shift ficam com a IDE;
- a serial mostra o que o projeto transmite, e o campo abaixo dela envia
  texto;
- **Tempo real** limita a velocidade à da placa; desmarcado, a simulação
  roda o mais rápido que der. O cabeçalho mostra o tempo simulado e a
  velocidade em relação ao tempo real.

A placa continua ligada quando você troca de aba, compila ou simula, e
desliga em **Desligar** ou ao abrir outro projeto. Se o projeto não compila
para a placa virtual, os erros aparecem na aba.

## 7.4 Controle de versão

O controle de versão é um recurso experimental e vem desligado. Ligue em
Preferências > Experimental > **Controle de versão**, ou com `aurora
experimental enable git` num terminal. Ele usa o git instalado na máquina
(sem ele, a vista mostra onde baixar o Git). Ligado, aparecem duas vistas na
barra lateral, a **Controle de versão** (Ctrl+Shift+G) e a **Perfil**, o ramo
na barra de status e as marcas de mudança no Explorador. As duas vistas vão
para a barra lateral esquerda ou para a direita como qualquer outra (botão
direito no ícone, ou Preferências > Layout da janela).

A vista **Controle de versão** segue o GitHub Desktop, e tudo abre dentro
dela:

- Fora de um repositório, **Criar repositório** cria um na pasta do
  projeto, com um `.gitignore` para o `.aurora8/` e as saídas dos
  processadores.
- No alto ficam o repositório, o ramo e o botão de sincronizar. Clicar no
  ramo abre a lista de ramos locais e remotos, com o filtro e **Criar
  ramo**; um ramo remoto vira um local que o acompanha. Se a troca apagaria
  mudanças que ainda não estão em commit, a lista mostra os arquivos. O
  botão de sincronizar muda conforme o ramo: **Publicar ramo** (o ramo
  ainda não está no remoto), **Trazer (n)**, **Enviar (n)** ou **Buscar**.
  Sem remoto, ele pede o endereço de um repositório vazio (no GitHub, em
  **Criar no GitHub**) e publica nele.
- **Mudanças** lista os arquivos mudados, com a letra da mudança (M
  modificado, U novo, A novo já no git, D apagado, R renomeado, ! conflito).
  Todos entram marcados no commit; desmarque o que fica de fora. Os de fora
  da pasta do projeto, num repositório maior que ela, ficam numa seção à
  parte. Clicar num arquivo abre a comparação com o último commit numa aba
  do editor: o lado direito é o próprio arquivo, e editar ali edita o
  arquivo (Ctrl+S grava). O botão de desfazer da linha, ou o botão direito,
  descarta as mudanças de um arquivo; a versão descartada vai para a
  lixeira do sistema. Embaixo, o resumo (obrigatório), a descrição e
  **Fazer commit em** o ramo; Ctrl+Enter também faz o commit. Sem nome e
  e-mail no git, a vista pede os dois antes do primeiro commit.
- **Histórico** lista os commits do ramo, com o autor, quando, e a marca
  **não enviado** dos que só existem nesta máquina. Clicar num commit mostra
  os arquivos dele; clicar num arquivo abre a comparação com o commit
  anterior, só para leitura.

Um commit, uma troca de ramo ou um fetch feitos num terminal aparecem na
vista sozinhos, quando o projeto é a pasta do repositório. Num repositório
maior que o projeto, use o botão de atualizar da vista.

A vista **Perfil** conecta a conta do GitHub ao git desta máquina. **Entrar
com o GitHub** mostra um código; **Copiar código e abrir o GitHub** abre a
página onde ele é digitado, e a vista espera a autorização. Depois disso, a
foto da conta aparece no ícone da vista, e o git envia e traz commits do
GitHub sem pedir senha, também no terminal. **Usar o nome e o e-mail do
GitHub** grava a identidade dos commits para todos os repositórios desta
máquina, com o e-mail do GitHub que não mostra o seu. **Sair** tira o login
do git desta máquina.

## 8. Ferramentas da AURORA

A aba **Ferramentas da AURORA** (chave inglesa na barra de atividades) é o
`aurora tools` com botões:

- o bundle, a plataforma, a pasta e como o Studio a achou;
- os componentes instalados e os que faltam, com **Instalar** (roda o
  `aurora install` da sua instalação; a saída aparece nesta tela, em
  **Saída de aurora install**);
- cada executável e onde está;
- o compilador do Verilator (do sistema no Linux e no macOS, do bundle no
  Windows);
- com a placa FPGA ligada (seção 7.2), o Quartus Prime, quando encontrado,
  com a versão e a pasta;
- **Conferir os hashes**: o `aurora tools --verify`;
- **Procurar atualizações**: o `aurora update --check`, com a versão
  instalada, a da última release e a upstream de cada componente. O
  resultado aparece no alto da tela;
- **Atualizar para X**: aparece depois da busca, quando há uma AURORA mais
  novo publicado. Pede confirmação e roda o `aurora update` da sua
  instalação, que baixa só os pedaços do bundle com arquivos que mudaram e
  troca esses arquivos no lugar, nos três sistemas; cada pedaço aparece
  nesta tela enquanto baixa, e no fim o tamanho baixado e os aplicativos
  que mudaram. Se a atualização por componentes não der certo, a
  instalação fica como estava e vem o instalador inteiro da release: no
  Linux e no macOS ele troca o `aurora` e o bundle com os mesmos
  componentes; no Windows, abre o assistente de instalação.

O Studio procura sozinho: uns segundos depois de abrir e, aberto, a cada 12
horas, ele roda o `aurora update --check` sem mostrar nada. Achando uma AURORA
mais novo, oferece a atualização num diálogo, com **Atualizar e reiniciar**
e **Depois**; se outro diálogo estiver aberto ou uma operação rodando, o
aviso vem no canto da janela. A oferta de cada versão aparece uma vez por
sessão, e o indicador **AURORA X disponível**, na barra de status, fica até a
atualização: o clique nele abre o diálogo de novo. Sem rede, a busca só
tenta de novo na próxima vez. Preferências > Geral > Procurar atualizações
desliga a busca automática; o botão desta tela continua valendo.

Atualizado, o Studio grava os arquivos abertos e as abas da sessão e
reinicia na versão nova; os terminais fecham. Ao abrir de novo, um aviso
diz para que versão ele foi. Se algum arquivo não gravar (um conflito com
o disco cancelado, por exemplo), o Studio não sai: grave o arquivo e
clique em **Reiniciar para concluir a atualização**, na barra de status.
No Windows, quando a atualização cai no assistente de instalação, o Studio
fecha para o assistente trocar os arquivos dele (um diálogo avisa antes), e
o assistente abre o Studio de novo quando a instalação termina.

A atualização não para no meio: o Parar some e o Shift+F5 não faz nada
enquanto ela roda, porque parar o `aurora` no meio da troca deixaria a
instalação pela metade. Um `aurora` que não veio do instalador (um build
local) não se atualiza por aqui, e a busca automática não oferece nada.
Um `aurora update` rodado num terminal, fora do Studio, não reinicia o
Studio aberto: ele vale depois de fechar e abrir o Studio.

## 9. Preferências

Ctrl+, ou a engrenagem. Tudo vale na hora e fica gravado. As páginas ficam
numa lista à esquerda (numa área estreita, numa faixa em cima, que as setas
do teclado também percorrem); a última aberta volta na próxima vez.

| Página | Preferência |
|---|---|
| Geral | idioma (do sistema, português, inglês), reabrir o último projeto, procurar atualizações ao abrir e a cada 12 horas (seção 8); no Windows, o shell do terminal: PowerShell ou Prompt de Comando (`cmd`) |
| Aparência | a interface (clássica ou em ilhas) e o tema (ver abaixo) |
| Layout da janela | o layout em uso (trocar, salvar e restaurar à vista; salvar como, renomear, excluir e voltar ao padrão no menu ⋯), as barras, as regiões, a posição do painel, as vistas de cada região (o olho mostra ou esconde; o menu ⋯ muda de região ou de ordem), o modelo da barra de ferramentas e os itens dela e da barra de status (seção 2.4) |
| Editor | tamanho da fonte, tabulação, quebra de linha, minimapa, salvar ao trocar de aba, modo Vim; e o grupo Modo zen: tela cheia, centralizar o editor, mostrar as abas, esconder os números de linha (seção 2.3) |
| Simulação | simulador padrão, onde abrir a onda (numa aba ou em janela separada), abrir a onda depois de simular (o botão Wave), prazo da simulação, consoles detalhados |
| Bundle da AURORA | a pasta do bundle (o `--toolchain` da CLI), a do compilador do Verilator (o `--compiler`), a pasta das trilhas de exercícios (o `AURORA_LEARN_DIR` da CLI; seção 7.1) e, com a placa FPGA ligada, a do Quartus Prime, que compila para as placas Intel (o `--quartus`) |
| Assistente | só com o Assistente de IA ligado (página Experimental): o executável do Claude Code, o modelo, se ele pode editar os arquivos do projeto e o executável `aurora` das ferramentas. Quem autentica é o Claude Code; o Studio não guarda chave |
| Experimental | os recursos da AURORA que ainda mudam muito, desligados por padrão: hoje, a **Placa FPGA** (seção 7.2) e o **Assistente de IA** (a vista Assistente, uma conversa com o Claude Code instalado na máquina sobre o projeto aberto, com as ferramentas da AURORA; ligado, ganha a própria página de Preferências). Ficam no `experimental.json` da pasta de configuração da AURORA (o caminho aparece na página), não no `settings.json`, e valem também para a CLI (`aurora experimental`) |

As preferências ficam em `settings.json`, na pasta de configuração do
aplicativo: `~/.config/com.nipscern.aurora-ide/` no Linux,
`%APPDATA%\com.nipscern.aurora-ide\` no Windows,
`~/Library/Application Support/com.nipscern.aurora-ide/` no macOS. O
caminho exato aparece em Ajuda > Sobre a AURORA IDE.

### Temas

Em Preferências > Aparência, a **Interface** escolhe o desenho da janela: a
Clássica, o padrão, com as regiões encostadas e separadas por linhas; ou
Ilhas, com as barras laterais, o editor e o painel em ilhas de cantos
redondos, separadas por vãos, as abas em pílula e o ícone ativo da barra de
atividades num quadrado tingido do destaque. As duas valem com qualquer
tema.

Logo abaixo, o tema tem duas escolhas: o modo (Escuro,
Claro ou Do sistema, que acompanha o sistema e troca junto com ele) e a
paleta, com um cartão por paleta e a prévia no modo escolhido. Clicar
aplica na hora. A paleta também se escolhe com Ctrl+K T (aperte Ctrl+K,
solte, aperte T) ou em Exibir > Selecionar tema, que abrem a lista na
paleta de comandos, e Exibir > Alternar tema claro e escuro troca o modo. O
tema pinta a interface, o editor (inclusive C± e o assembly) e os consoles.

| Paleta | Modos | Origem |
|---|---|---|
| Boreal (padrão) | escuro e claro | os da AURORA IDE: cinzas quase neutros com o azul da aurora boreal de destaque |
| Austral | escuro e claro | os mesmos cinzas, com o verde da Aurora antiga de destaque e o lilás dela nos números e textos |
| Nebulosa | escuro e claro | os mesmos cinzas, com o violeta da Aurora antiga de destaque e a menta dela nos números e textos |
| Alvorada | escuro e claro | os mesmos cinzas, com o âmbar do amanhecer |
| Lunar | escuro e claro | os mesmos cinzas, com a interface no prata do texto; no código, azul de luar nas palavras reservadas (em negrito), lilás prateado nos tipos e dourado nos números e textos |
| Aurora Legacy | só escuro | as cores da AURORA |
| VS Code | escuro e claro | o Dark Modern e o Light Modern, os padrões do VS Code |
| Dracula | só escuro | |

Uma paleta só escura fica escura em qualquer modo.

### Ícones de arquivo

No explorador e nas abas, os arquivos do fluxo da AURORA têm ícone próprio,
cada tipo na cor dele no tema:

| Arquivo | Ícone |
|---|---|
| Verilog (`.v`, `.sv`, `.vh`, `.svh`) | a caixa de um módulo, com as entradas e a saída |
| Testbench (os da lista do projeto, `tb_<nome>.v`, `<nome>_tb.v`, `test_<nome>.py`) | um frasco de laboratório |
| C± (`.cmm`) | C± |
| Assembly (`.asm`) | 01 |
| Memória (`.mif`) | uma tabela de células |
| Onda (`.vcd`, `.fst`, `.ghw`) | uma onda quadrada |
| Projeto (`.spf`) | um selo com uma estrela |

## 9.1 Sobre

Ajuda > Sobre a AURORA IDE abre uma aba com a versão da IDE, a do bundle da AURORA e a
de cada componente (com os que faltam instalar), as do Tauri e do motor de
páginas do sistema (WebKitGTK, WebView2 ou WKWebView), as pastas das
preferências e do bundle (o botão ao lado abre a pasta) e os links da
documentação da AURORA, do manual do SAPHO e do NIPS-CERN. **Copiar
informações** põe tudo isso, em inglês, na área de transferência, para
anexar a um relato de problema.

## 10. Atalhos

No macOS, Ctrl é Cmd. A lista também está em Ajuda > Atalhos de teclado.

| Ação | Teclas |
|---|---|
| Paleta de comandos | Ctrl+Shift+P |
| Abrir arquivo do projeto pelo nome | Ctrl+P |
| Novo projeto, abrir projeto | Ctrl+Alt+N, Ctrl+Shift+O |
| Novo processador | Ctrl+Alt+P |
| Novo arquivo, abrir arquivo | Ctrl+N, Ctrl+O |
| Salvar, salvar todos | Ctrl+S, Ctrl+Shift+S |
| Fechar aba, reabrir aba fechada | Ctrl+W, Ctrl+Shift+T |
| Dividir o editor, ir para o grupo 1, 2, 3 | `Ctrl+\`, Ctrl+1, Ctrl+2, Ctrl+3 |
| Abrir ao lado (na árvore do explorador) | Ctrl+Enter |
| Preferências | Ctrl+, |
| C±, Verilog, lint | F6, F7, Shift+F7 |
| Wave, Rápida, abrir a onda | F8, F9, Ctrl+F8 |
| RTLViewer | F10 |
| Fluxo completo, Parar | F5, Shift+F5 |
| Localizar, substituir, ir para a linha | Ctrl+F, Ctrl+H, Ctrl+G |
| Localizar nos arquivos | Ctrl+Shift+F |
| Formatar documento | Shift+Alt+F |
| Explorador, barra lateral esquerda, barra lateral direita, painel | Ctrl+Shift+E, Ctrl+B, Ctrl+Alt+B, Ctrl+J |
| Selecionar layout | Ctrl+K L |
| Selecionar tema | Ctrl+K T |
| INFO, Terminal, Problemas | Ctrl+Shift+U, Ctrl+`, Ctrl+Shift+M |
| Controle de versão (com o recurso ligado) | Ctrl+Shift+G |
| Fazer commit (na caixa de commit) | Ctrl+Enter |
| Modo zen (entrar e sair) | Ctrl+K Z; Esc Esc também sai |
| Histórico de relatórios | Ctrl+Shift+H |
| Corrigir o exercício (aurora learn) | Ctrl+Alt+L |
| Zoom | Ctrl+=, Ctrl+-, Ctrl+0 |

Dentro do Terminal, as teclas com Ctrl sozinho (Ctrl+C, Ctrl+W, Ctrl+P) vão
para o shell; só as teclas de função, as combinações com Ctrl+Shift e o
Ctrl+` continuam do Studio.

O Ctrl+F busca no arquivo da frente. Com o RTLViewer na frente, vai para a
busca dele; sem arquivo nem RTLViewer (outra vista, o editor vazio), abre a
Busca do projeto, como Ctrl+Shift+F.

### Modo Vim

Editar > Alternar modo Vim, ou a caixa nas Preferências. O editor passa a
usar as teclas do Vim (o `monaco-vim`, que adapta as do CodeMirror), e a
barra de status mostra o modo (`--NORMAL--`, `--INSERT--`, `--VISUAL--`) e
a linha de comando. Além dos comandos do Vim:

| Comando | Faz |
|---|---|
| `:w` | grava a aba |
| `:wa` | grava todas |
| `:q` | fecha a aba |
| `:wq`, `:x` | gravam e fecham |

Com o Vim ligado e o foco no editor, Ctrl+B e Ctrl+O ficam com o Vim
(página para cima, voltar no histórico de saltos) em vez de esconder a
barra lateral e abrir arquivo. O Ctrl+F não: ele abre a busca no arquivo, e
não desce uma página como no Vim (Page Down desce). Os outros atalhos do
Studio continuam. No
modo zen, a linha do Vim aparece no indicador do canto, porque a barra de
status está escondida; Esc duas vezes não sai do zen, Ctrl+K Z sai.

## 11. Problemas comuns

| Sintoma | O que fazer |
|---|---|
| "AURORA não encontrada" | instale a AURORA, ou indique a pasta do bundle em Preferências > Bundle da AURORA (a que tem `bundle.json`) |
| "Falta um componente do bundle" | Ferramentas da AURORA > Não instalados > Instalar |
| A simulação não termina | o testbench chega ao `$finish`? Use Parar, ou defina um prazo em Preferências > Simulação |
| "O projeto não tem módulo de topo" | botão direito num módulo, no explorador > Definir como topo; ou Projeto > Escolher o módulo de topo |
| "O processador ainda não foi compilado" | C± (F6) com o processador no alvo |
| O Verilator falha por falta de compilador | instale g++ (ou clang++), make e Perl; ou indique a pasta deles em Preferências > Bundle da AURORA > Compilador do Verilator |
| O surfer-aurora fecha logo ao abrir | sem display gráfico, ou onda inválida; o log dele fica em `.aurora8/Temp/surfer/<onda>.log` |
| A aba da onda diz que o bundle não traz o cliente web | o bundle é anterior ao cliente web do Surfer; a onda abre em janela até um bundle novo |
| A aba da onda recusa uma onda grande | acima de 256 MB a onda abre em janela (o ícone de janela, no alto da aba) |
| "O arquivo mudou no disco" ao salvar | outro programa alterou o arquivo; escolha sobrescrever ou recarregar |
| "O fonte do processador não está como o YANC espera" | o arquivo precisa se chamar `<processador>.cmm` (ou `.cpp`) |
| "Este processador ainda está na estrutura de pastas do YANC 6.x" | converta o projeto: **Arquivo > Converter projeto para o YANC 7.1** |
| "O caminho é longo demais para o YANC" | no Windows o YANC aceita até 259 caracteres; mova o projeto para uma pasta mais curta |
