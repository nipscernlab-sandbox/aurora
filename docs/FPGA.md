# Placas FPGA

A AURORA leva um projeto para uma placa FPGA: confere as ligações das portas
do topo aos sinais da placa, gera o topo da placa, compila pelo Quartus
Prime, sem abrir a interface dele, e grava pelo cabo USB da placa. Este
documento descreve as placas, o `fpga.json`, `aurora fpga boards`, `aurora fpga
check`, `aurora fpga build`, `aurora fpga program`, a detecção do Quartus e o
componente `openfpgaloader`.

**É um componente experimental, desligado por padrão.** Ligue com:

```
aurora experimental enable fpga
```

ou no Studio, em Preferências > Experimental > Placa FPGA. O que se liga
num vale para o outro: os dois leem o `experimental.json` da pasta de
configuração da AURORA ([CLI.md](CLI.md), "Componentes experimentais").
Desligado, todo `aurora fpga` sai com 2 e diz como ligar
(`experimental_disabled`), e o Studio tira da janela a aba Placa FPGA, o
botão FPGA da barra de ferramentas, as ações do menu Fluxo e da paleta, a
seção FPGA do navegador de fluxo, o console Placa e o Quartus Prime das
Preferências e da tela do bundle; os comandos da placa no backend do
Studio respondem `experimental_disabled`. O `fpga.json` e a pasta
`.aurora8/fpga/` do projeto ficam como estão.

## Placas

| Placa | `board` | FPGA | Cabo |
|---|---|---|---|
| Terasic DE2-115 | `de2-115` | Cyclone IV E EP4CE115F29C7 | USB-Blaster |
| Terasic DE10-Nano | `de10-nano` | Cyclone V SE 5CSEBA6U23I7 | USB-Blaster II |
| Terasic Cyclone V GX Starter Kit | `c5g` | Cyclone V GX 5CGXFC5C6F27C7 | USB-Blaster |

Os pinos vêm dos manuais da Terasic; cada placa cita o manual, a versão e as
tabelas no campo `source`. Em 2026-10-09 os pinos e os padrões de I/O das
duas placas foram comparados, um a um, com o `.qsf` que a própria Terasic
distribui ("golden top", gerado pelo System Builder): 110 de 110 na DE2-115
e 17 de 17 na DE10-Nano, sem diferença. Os LEDs 6 e 7 da DE10-Nano, que a
tabela do manual não lista, vêm desse arquivo. Em 2026-10-13 os 65 pinos
da C5G foram comparados do mesmo jeito com dois arquivos do CD dela
(v1.3.0), o golden top e o da demonstração de fábrica: 65 de 65 nos dois.
Em 2026-11-05 entraram na DE2-115 a VGA (tabela 4-16 do manual v2.3), a
serial RS-232 (tabela 4-18, com as direções da figura 4-24) e o PS/2
(tabela 4-19), 37 pinos lidos do manual e ainda não comparados com o golden
top. `aurora fpga boards` lista as
placas, e `aurora fpga boards <placa>` mostra os sinais com os pinos, o padrão
de I/O e quais são ativos em nível baixo. Com `--json`, os dados completos.

Os dados de cada placa ficam em `crates/aurora-core/boards/<placa>.json`,
embutidos na AURORA. Uma placa nova é um arquivo novo nessa pasta e uma linha
em `board.rs`; os testes conferem nomes, pinos repetidos e clocks.

**Na C5G** (Cyclone V GX Starter Kit), as chaves SW, os botões KEY e os
displays HEX0 e HEX1 são de 1,2 V, e os LEDs, de 2,5 V. HEX2 e HEX3 são de
3,3 V e dividem os pinos com o conector GPIO de 40 pinos (GPIO22 a
GPIO35): só acendem com as chaves DIP S1 e S2, no verso da placa, em ON.
Com HEX2 ou HEX3 no `fpga.json`, a FPGA dirige esses pinos do GPIO, então
nada ligado neles pode dirigi-los também; sem eles, os pinos ficam como
entrada com pull-up fraco, como todo pino sem uso. O `CPU_RESET_n` (KEY4)
é um botão à parte, de 3,3 V, que o manual não põe no circuito de
debounce. Os clocks diferenciais (o de 125 MHz e os de referência dos
transceptores) ficam fora do perfil. Para gravar, a chave RUN/PROG (SW11)
fica em RUN, e o jumper JP2 fechado nos pinos 1 e 2 deixa a FPGA sozinha
na cadeia JTAG. O manual chama o cabo da placa de USB-Blaster, e duas
demonstrações do CD falam em USB-Blaster II; a gravação pelo Quartus usa
o primeiro cabo que o `quartus_pgm -l` lista, de qualquer tipo.

## O `fpga.json`

Fica na raiz do projeto, ao lado do `.spf`. A AURORA 7, a IDE em Electron,
não o lê. Na AURORA IDE, a tabela da aba **Placa FPGA** e o modo de editar
as ligações da **Placa virtual** o gravam; à mão, é um JSON comum.

```json
{
  "board": "de2-115",
  "top": "media_movel",
  "connect": {
    "clk": "CLOCK_50",
    "rst": "!KEY[0]",
    "in[3:0]": "SW[3:0]",
    "out[7:0]": "LEDR[7:0]"
  }
}
```

- `board`: a placa.
- `top`: o módulo que vai para a placa. Sem ele, o topo do projeto; sem
  topo, o processador, se o projeto tiver um só.
- `connect`: cada porta do topo (com faixa ou não) e o sinal da placa que
  liga nela.

Cada ligação é `porta[faixa]` → `[!]SINAL[faixa]`. Uma entrada do topo
também pode receber `0` ou `1`. O `!` inverte o sinal: o `rst` do SAPHO é
ativo em nível alto e os botões das placas da Terasic, em nível baixo, então
`"rst": "!KEY[0]"` dá reset com o botão apertado. As faixas vão do bit mais
alto ao mais baixo (`[7:0]`) ou são um bit só (`[3]`); sem faixa, vale a
porta ou o sinal inteiro.

Regras:

- Uma entrada do topo só recebe sinal de entrada da placa (clock, botão,
  chave); uma saída do topo só dirige sinal de saída da placa (LED,
  display). Porta `inout` não é ligada.
- Com faixa dos dois lados, as larguras têm de ser iguais. Com faixa de um
  lado só, ou de nenhum, ligam-se os bits de baixo: a entrada que sobra
  recebe 0 e a saída que sobra não chega à placa. A AURORA avisa cada ajuste.
- Um bit de entrada do topo tem uma ligação só; um bit de saída da placa
  também. Uma saída do topo pode dirigir mais de um sinal da placa: numa
  lista (`"seg": ["HEX0", "HEX1"]` acende os dois displays) ou em faixas
  que se sobrepõem (`"LEDR": "LEDR"` e `"LEDR[6:0]": "HEX4"`). A mesma
  porta duas vezes no objeto é erro, porque o JSON guardaria só uma.
- Entrada do topo sem ligação recebe 0, com aviso. Bit de saída da placa
  sem ligação fica no nível inativo: apagado (0) num LED, apagado (1) num
  segmento de display, que é ativo em nível baixo.

`aurora fpga check` confere o `fpga.json` contra a placa e as portas do topo
e mostra o que vai em cada porta e em cada sinal da placa. Todos os
problemas aparecem de uma vez, um por linha:

```
$ aurora fpga check
OK media_movel on Terasic DE2-115 (EP4CE115F29C7)
  clk         <-  CLOCK_50
  rst         <-  ~KEY[0]
  in[15:0]    <-  {12'b0, SW[3:0]}
  LEDR[17:0]  <-  {10'b0, out[7:0]}
  clock CLOCK_50 (50 MHz) on clk
  note: in[15:4] is tied to 0 (no connection)
```

Com `"in": "SW"` (sem faixa dos dois lados), as 16 chaves de baixo iriam
para `in` e não haveria nota; com `"in": "SW[3:0]"`, a nota seria
`in[15:4] receives 0 (SW[3:0] has 4 bits)`.

As portas do topo vêm do Verilog: um processador SAPHO precisa estar
compilado (`aurora build`).

## O topo da placa

A AURORA gera um módulo, `aurora_board_top`, que tem como portas só os sinais
da placa usados e instancia o topo do projeto com as ligações. É ele que
vai para o Quartus, com um pino por bit de cada sinal. `aurora fpga check --show-top`
mostra o Verilog. Ele é gerado de novo a cada compilação: não edite.

O topo do projeto pode ser uma entidade VHDL. O Quartus lê o VHDL direto
(`VHDL_FILE` no `.qsf`, em VHDL-2008) e o mistura com o topo da placa, que
continua em Verilog e instancia a entidade pelo nome em minúsculas. As
portas vêm do leitor de VHDL da AURORA, com os genéricos nos valores
padrão: `std_logic`, `std_ulogic`, `bit` e `boolean` têm um bit, e
`std_logic_vector`, `std_ulogic_vector`, `bit_vector`, `unsigned` e
`signed` têm os bits do intervalo (`WIDTH - 1 downto 0`). Uma porta de
outro tipo (um `integer`, um tipo enumerado, um registro) não vira pinos:
`fpga check` recusa com o nome dela. Na placa virtual, o Verilator recebe o
Verilog que o `ghdl --synth` gera do VHDL do projeto. Conferido em
2026-11-10 com um contador VHDL na DE2-115: o Quartus 25.1 compilou e fechou
o tempo, e a placa virtual rodou.

## O Quartus Prime

Para as placas Intel, o bitstream sai do Quartus Prime, que não tem
alternativa aberta para o Cyclone IV. A AURORA roda o Quartus sem abrir a
interface. Ele não vai no bundle (é proprietário e grande): é a segunda
exceção à regra do bundle, depois do compilador do Verilator. A AURORA o
procura nesta ordem:

1. `--quartus <DIR>` ou `AURORA_QUARTUS`: a pasta da versão
   (`C:\intelFPGA_lite\22.1std`), a `quartus` dentro dela ou a dos
   programas. Pasta declarada sem o `quartus_sh` é erro.
2. `QUARTUS_ROOTDIR`, que o instalador da Intel cria.
3. As pastas padrão do instalador: `C:\intelFPGA_lite`, `C:\intelFPGA`,
   `C:\altera_lite` e `C:\altera` no Windows; as mesmas na pasta do usuário
   e em `/opt` no Linux. Com mais de uma versão, a mais nova.

`aurora tools` mostra o Quartus encontrado. O Quartus não existe para macOS.

A instalação precisa do Quartus Prime Lite e do suporte à família da placa,
que a Intel distribui à parte: Cyclone IV para a DE2-115 e Cyclone V para a
DE10-Nano e a C5G. O Questa (simulador) e as outras famílias não são usados.

## Compilar: `aurora fpga build`

```
aurora fpga build
```

1. Confere o `fpga.json`, a placa e o Quartus antes de compilar qualquer
   coisa: faltar um deles não custa o build dos processadores.
2. Compila os processadores com o YANC, como `aurora synth`. O Verilog de um
   processador cita as memórias (`.mif`) pelo nome, como o YANC 7.1 gera,
   e elas vão para a pasta da placa, a do projeto do Quartus, antes da
   compilação. Que o Quartus as procura ali é suposição, ainda não
   conferida numa compilação.
3. Confere as ligações (`aurora fpga check`) e grava o projeto do Quartus na
   pasta da placa.
4. Roda os quatro programas que a interface do Quartus roda ao compilar,
   um por passo.

```text
<raiz>/.aurora8/fpga/<placa>/
  aurora_board_top.v      o topo da placa
  aurora_board_top.qsf    a FPGA, os fontes e os pinos
  aurora_board_top.sdc    os clocks da placa
  output_files/         o .sof, o .rbf, o .svf e os relatórios do Quartus
  db/, incremental_db/  o banco de dados do Quartus

synthesize  quartus_map --read_settings_files=on --write_settings_files=off aurora_board_top -c aurora_board_top
fit         quartus_fit --read_settings_files=off --write_settings_files=off aurora_board_top -c aurora_board_top
bitstream   quartus_asm --read_settings_files=off --write_settings_files=off aurora_board_top -c aurora_board_top
timing      quartus_sta aurora_board_top -c aurora_board_top
```

O `.qsf` traz a família e o modelo da FPGA, os fontes (a biblioteca SAPHO,
se o projeto tem processadores, os arquivos do projeto e o topo da placa),
a raiz do projeto como pasta de `include` e um pino por bit de cada sinal
usado, com o padrão de I/O do manual. Os pinos sem uso ficam como entrada,
em alta impedância, com o pull-up fraco: é o padrão do Quartus, escrito no
`.qsf` para não depender da versão. Pede também o `.rbf` e o `.svf` sem
compressão, as três linhas que a documentação do openFPGALoader indica: pela
JTAG a FPGA não descomprime. O `.sdc` cria um clock por sinal de clock da
placa ligado ao topo, com o período do oscilador. Os três arquivos são
gravados de novo a cada compilação, em inglês e só com ASCII, por
precaução: não se sabe em que codificação o Quartus os lê.

Depois do Fitter, antes de gerar o arquivo de gravação, a AURORA lê o `.pin`
(onde o Quartus de fato pôs cada sinal) e confere cada bit contra a placa:
o pino, a direção, o padrão de I/O e que a posição veio da atribuição, e
não de uma escolha do Fitter. Qualquer diferença reprova a compilação no
passo `fit`, com uma mensagem por bit, e o `.sof` não é gerado.

No fim, a compilação grava o `aurora-build.json` na pasta da placa: o SHA-256
do `.sof` e de tudo o que entrou nele (os fontes, a biblioteca SAPHO, as
memórias `.mif` dos processadores, o `fpga.json` e a definição da placa).
É por ele que a gravação sabe se o `.sof` ainda descreve o projeto.

Depois de compilar, a AURORA mostra os erros e avisos do Quartus com arquivo e
linha (os `Info`, centenas, ficam de fora; o aviso repetido em cada canto de
tempo aparece uma vez), os recursos da FPGA do resumo do Fitter e, por
clock, a frequência pedida, a Fmax e as folgas de setup e de hold no pior
canto. Folga negativa não reprova a compilação: o Quartus gera o `.sof` do
mesmo jeito, e a AURORA avisa que o design pode falhar na placa nessa
frequência.

Saída real de `aurora fpga build` no projeto `bcd_prot`, com o Quartus Prime
25.1 Lite (o começo, com o build e os passos, foi cortado):

```
  quartus: warning: 1 pins must meet Intel FPGA requirements for 3.3-, 3.0-, and 2.5-V interfaces. For more information, refer to AN 447: Interfacing Cyclone IV E Devices with 3.3/3.0/2.5-V LVTTL/LVCMOS I/O Systems. [169177]
  Generated:
    bitstream (.sof)    .aurora8\fpga\de2-115\output_files\aurora_board_top.sof
  Resources:
    Total logic elements                677 / 114480 (<1%)
    Total registers                     39
    Total pins                          67 / 529 (13%)
    Total virtual pins                  0
    Total memory bits                   0 / 3981312 (0%)
    Embedded Multiplier 9-bit elements  2 / 532 (<1%)
    Total PLLs                          0 / 4 (0%)
  Timing: met
    CLOCK_50  needs 50 MHz, reaches 250 MHz, setup slack 16.985 ns, hold slack 0.192 ns
  Program the Terasic DE2-115 with: aurora fpga program
```

O aviso 169177 vem com a submensagem `Pin CLOCK_50 uses I/O standard 3.3-V
LVTTL at Y2`: é o oscilador de 50 MHz da placa, com o mesmo padrão que o
`.qsf` da Terasic usa, e o Quartus lembra os requisitos de interfaces de
3,3 V (a nota de aplicação AN 447).

O número entre colchetes é o da mensagem do Quartus, que é o que se procura
na base de conhecimento da Intel; `critical` marca os avisos críticos. Com
`--json`, o relatório `fpga-build` (`docs/schema/fpga-build.json`).

Conferido em 2026-10-09 com o Quartus Prime 25.1 Lite no Windows e uma
DE2-115, compilando e gravando. Ainda não conferido:

- a DE10-Nano com o Quartus, e o Quartus no Linux, em que as variáveis de
  ambiente que a AURORA passa (a pasta e o nome do usuário, o idioma e a pasta
  temporária) são dedução; o relatório de cada passo mostra quais foram;
- caminhos com acento: o simulador já os recusa num projeto com
  processadores; no Quartus, não se sabe.

## Gravar: `aurora fpga program`

```
aurora fpga program            # o primeiro cabo ligado
aurora fpga program --list     # os cabos que o Quartus vê
```

Grava o `.sof` da última `aurora fpga build` pelo Quartus Programmer, na
pasta da placa:

```
quartus_pgm -c "USB-Blaster [USB-0]" -m jtag -o "p;output_files/aurora_board_top.sof@1"
```

O `@1` é a posição da FPGA na cadeia JTAG da placa: 1 na DE2-115 e na C5G
(com o JP2 fechado), 2 na DE10-Nano, em que o HPS vem antes. A gravação vai para a SRAM da FPGA e
some quando a placa é desligada.

Antes de procurar o cabo, a gravação confere o `aurora-build.json` contra o
projeto de agora, pelo conteúdo: um fonte, uma memória, o `fpga.json` ou a
definição da placa que mudou depois da compilação, um `.sof` trocado ou uma
compilação que não terminou recusam a gravação (`stale_bitstream`), com
cada diferença. Regravar os mesmos arquivos (um `aurora build` que gera o
mesmo Verilog) não conta como mudança.

Conferido em 2026-10-09 com o Quartus Prime 25.1 Lite no Windows e uma
DE2-115: o projeto `bcd_prot` compilou em 35 s e gravou em 9 s. Antes de
gravar pela primeira vez:

- ligue a placa pela porta USB BLASTER, com a chave RUN/PROG em RUN;
- no Windows, o driver do USB-Blaster está na pasta `drivers` do Quartus
  (`quartus/drivers/usb-blaster`); o Gerenciador de Dispositivos instala
  por ele.

Sem cabo, o erro é `no_cable`; sem a compilação, `no_bitstream`.

No Linux, não testado: a AURORA procura o Quartus em `~/intelFPGA_lite`,
`~/intelFPGA`, `~/altera_lite`, `~/altera` e nas mesmas pastas em `/opt`, e
roda os programas de `quartus/bin`. O código compila para Linux sem aviso.
Para gravar, o Quartus no Linux pede uma regra do udev que dê ao usuário
acesso ao USB-Blaster (sem ela, o `quartus_pgm -l` não vê o cabo e o erro é
`no_cable`); a regra e o serviço `jtagd` são os da documentação da Intel
para o cabo no Linux, e pedem root.

## Segurança da placa

Conferido em 2026-10-09 na DE2-115, com os relatórios da compilação real do
`bcd_prot` e o manual da placa:

| Risco | O que impede |
|---|---|
| Uma saída da FPGA num pino que a placa liga a uma chave, a um botão ou ao oscilador: curto quando os dois lados divergem | Cada sinal da placa tem direção no arquivo dela, e a AURORA só liga uma saída do topo a um sinal de saída da placa (LEDs, displays). Porta `inout` não é ligada. |
| Pino errado no arquivo da placa | Os pinos e os padrões de I/O foram conferidos com o manual e com o `.qsf` da Terasic, um a um. |
| Uma atribuição que o Quartus não aplica, e o Fitter escolhe o pino | A conferência do `.pin` depois do Fitter reprova a compilação antes do `.sof`. |
| Um pino sem uso dirigindo o que está ligado nele | Os pinos sem uso ficam como entrada em alta impedância, com pull-up fraco, escrito no `.qsf`. No `.pin` da compilação real: 447 pinos `RESERVED_INPUT_WITH_WEAK_PULLUP`. |
| Gravar um design diferente do que está no projeto | A gravação recusa um `.sof` velho, trocado ou de outra placa (`stale_bitstream`). |
| Perder o projeto de fábrica da placa | A gravação é pela JTAG, direto na FPGA (`.sof`), e some ao desligar. A memória EPCS64, que guarda o projeto que a placa carrega ao ligar, só é gravada com a chave em PROG e um `.pof` (manual da DE2-115, seção 4.1); a AURORA nunca faz isso. |

Com a chave JP3 na posição de fábrica (pinos 1 e 2), a cadeia JTAG da
DE2-115 tem só a FPGA, e o `@1` da gravação é ela. Na gravação real, o
Quartus Programmer leu o código JTAG da FPGA (`Device 1 contains JTAG ID
code 0x020F70DD`) antes de configurar.

O que a AURORA não detecta: um jumper mudado. Os padrões de I/O das chaves,
dos botões, dos LEDs e de parte dos displays seguem o JP7 de fábrica (2,5 V
nos bancos 5 e 6); com o JP7 em outra tensão, esses padrões deixam de
corresponder à placa.

## Gravação: o componente `openfpgaloader`

O openFPGALoader grava um `.rbf` ou um `.svf` pelo cabo da placa. Vem do
OSS CAD Suite, nas três plataformas, no componente opcional
`openfpgaloader` (`aurora install openfpgaloader`).

- Na DE10-Nano, o USB-Blaster II só funciona no openFPGALoader com o
  firmware `blaster_6810.hex`, que vem com o Quartus: gravar nela pede o
  Quartus instalado.
- No Linux, o acesso ao cabo pede uma regra do udev, que precisa de root. O
  OSS CAD Suite não traz a do openFPGALoader.
- No Windows, o driver do USB-Blaster que o Quartus instala não é o que o
  openFPGALoader usa (libusb). Por isso a AURORA grava só pelo `quartus_pgm`,
  e sem o Quartus a gravação dá `quartus_missing`. Falta testar com as
  placas.

## Placa virtual: `aurora vboard`

A placa virtual roda o projeto no computador com os periféricos da placa
simulados nos pinos, antes de gravar a placa de verdade. O topo que ela
simula é o topo da placa do fluxo acima: o mesmo `fpga.json`, as mesmas
ligações e o mesmo `aurora_board_top`. Num projeto sem `fpga.json`, `--board`
(ou a escolha da placa na IDE) liga cada porta do topo que tem o nome e a
direção de um sinal da placa (`SW`, `KEY`, `LEDR`, `HEX0`), como nos topos de
exemplo da Terasic; o `fpga.json` não é gravado.

O que cada sinal é na placa vem da lista `peripherals` do JSON dela:

| Tipo | Sinais | O que a placa virtual faz |
|---|---|---|
| `switches` | `signal` | o valor de cada chave |
| `buttons` | `signal` | o botão apertado vai a 0 num sinal ativo em baixo (os `KEY`) |
| `leds` | `signal`, `color` | o tempo aceso de cada LED, de 0 a 255: o PWM aparece como brilho |
| `seven_segment` | `digits`, da esquerda para a direita | cada segmento como um LED (bit 0 = a, bit 6 = g, bit 7 = ponto); o display multiplexado aparece pela persistência |
| `vga` | `red`, `green`, `blue`, `hsync`, `vsync`, `blank` e `clock` opcionais | a imagem remontada pelo sincronismo, na subida do relógio de vídeo (`VGA_CLK`) ou do relógio da placa; com `blank` (ou o DE de um transmissor HDMI) a área visível vem dele, sem ele da tabela de modos VESA (640x480 a 1920x1080) |
| `ps2` | `clock`, `data` | as teclas do computador como quadros PS/2 do conjunto 2, a 12,5 kHz em tempo simulado; nada volta do projeto para o teclado |
| `uart` | `tx`, `rx` opcional, `baud` | os bytes que o projeto transmite e os que ele recebe, na velocidade dada |

As três placas têm chaves, botões, LEDs e displays. A DE2-115 tem também a
VGA, o primeiro conector PS/2 (`PS2_CLK`, `PS2_DAT`) e a serial (`UART_TXD`,
`UART_RXD`, a 115200 bps); o segundo PS/2 e o `UART_CTS`/`UART_RTS` estão no
JSON dela para o Quartus, mas a placa virtual não os simula. O vídeo para o
transmissor HDMI da C5G e da DE10-Nano entra quando os pinos forem
conferidos no manual. Com os blocos `myKeyPS2` e `myVGAText` da biblioteca
SAPHO ligados um no outro na DE2-115, o que se digita aparece na tela, sai
pela serial e conta nos displays (2026-11-05, pela CLI e pela IDE).

`aurora vboard run` compila e roda sem janela (a janela é a aba **Placa
virtual** da IDE):

```
$ aurora vboard run --board de2-115 --set SW=0x2A5 --press KEY=0b10 --time 30
Virtual board: placa on Terasic DE2-115 (verilator --cc --exe --build): finished in 16.81 s
Terasic DE2-115 30.188 ms simulated in 0.3 s
  LEDR         000000001010100101
  LEDG         000000000
  HEX7               15
```

O Verilator do bundle compila o topo da placa, os fontes do projeto (com o
Verilog e as memórias de cada processador SAPHO compilado) e um laço de
simulação em C++ que simula os periféricos, em `.aurora8/Temp/vboard`, e
roda na raiz do projeto, como a simulação: um `$readmemb("program.txt")`
do projeto acha o arquivo. No
Linux e no macOS ele usa o compilador C++ do sistema, como a simulação com
o Verilator. A primeira compilação leva de 15 a 30 segundos; as seguintes
só refazem o que mudou. Medido nesta máquina (Windows, 2026-11-05): um
projeto pequeno roda a cerca de 10 milhões de flancos de relógio por
segundo, um décimo do tempo real a 50 MHz, e a tela de texto do `myVGAText`
sozinha, a 25 quadros por segundo. Um projeto maior roda mais devagar. A
opção de tempo real (o padrão na IDE) limita a velocidade quando o projeto
roda mais rápido que a placa.

Na IDE, a aba **Placa virtual** mostra, em cima, o monitor, a serial e o
teclado que o topo usa (os outros ficam numa linha, sem ocupar a página) e,
embaixo, a placa: os displays, os LEDs sobre as chaves de mesmo bit e os
botões. Cada elemento mostra a porta do topo ligada a ele, em faixas
(`SW[17:3]`, `KEY[1]`). Com a placa desligada, **Editar ligações** faz cada
chave, botão, LED e display escolher a porta que vai nele: o núcleo
(`fpga.connect`) muda só aquele bit (ou aquele display inteiro), deixa as
outras ligações como estavam e grava o `fpga.json`, reescrito em faixas
contínuas. Uma saída que já ia a outro lugar da placa continua lá. A tela, o
teclado e a serial, que têm vários sinais, se ligam na tabela da aba
**Placa FPGA**.

Limites: um bloco de IP da Intel (PLL, memória com `.mif`, DDIO) não simula
no Verilator; o Verilator é mais rigoroso que o Icarus, e um projeto que
simula no Icarus pode não compilar; `$display` do projeto aparece na saída,
mas o tempo simulado não chega ao `$time`.

## O que falta

- Comparar os 37 pinos novos da DE2-115 (VGA, PS/2, serial) com o golden
  top da Terasic, como os outros 110, e gravar uma DE2-115 com eles.
- Os pinos do vídeo HDMI da C5G e da DE10-Nano no JSON das placas,
  conferidos no manual, para a placa virtual usá-los.
- Na placa virtual: o LCD de caracteres e a SRAM da DE2-115, o áudio, o
  segundo PS/2, os comandos do projeto para o teclado PS/2 e a ligação
  direta aos `toma` e `cade` do SAPHO, sem simular a varredura da tela.

- Compilar e gravar a DE10-Nano com o Quartus de verdade (a DE2-115 foi
  conferida).
- Gravar uma C5G de verdade. O perfil compila no Quartus 25.1 (os 65 pinos
  no 5CGXFC5C6F27C7, com o timing atendido nos quatro clocks), mas nenhuma
  C5G foi gravada, e o tipo do cabo (USB-Blaster ou USB-Blaster II) falta
  conferir com a placa ligada.
- Gravar pelo openFPGALoader, sem o Quartus.
- Guardar a compilação para a placa no histórico (`aurora report`).
- O fluxo aberto para o Cyclone V (Yosys e `nextpnr-mistral`), adiado: o
  `nextpnr-mistral` não vem no OSS CAD Suite 2026-10-22, em nenhuma
  plataforma, então teria de ser compilado pela AURORA.
