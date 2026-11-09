; Instalador da AURORA para Windows (Inno Setup 6).
;
; O workflow de release compila dois assistentes deste script:
;
;   iscc /DStage=<estágio> /DAuroraVersion=0.1.0 /DBundle=2026.09.29 /O<saída> installer\windows\aurora.iss
;   iscc /DWeb /DStage=<estágio> /DAuroraVersion=0.1.0 /DBundle=2026.09.29 /O<saída> installer\windows\aurora.iss
;
; O estágio vem de `aurora-pack inno`: bin\aurora.exe, common\ e chunks\NN\ com
; os arquivos do bundle, e components.iss com as seções [Components] e
; [Files], geradas do índice do bundle (que arquivo cada componente usa; as
; bibliotecas que o Icarus, o Verilator, o Yosys e o Graphviz dividem vão
; com qualquer um deles).
;
; O primeiro assistente leva o bundle inteiro dentro dele. O segundo (/DWeb,
; com o web.iss que o `aurora-pack inno --web` grava) leva só o aurora.exe: ao
; clicar em Instalar, ele baixa da release o índice e os pedaços dos
; componentes marcados, cada um conferido pelo SHA-256, e o `aurora setup` os
; extrai e troca o toolchain\ antes de o assistente copiar o aurora.exe. A
; release vem do GitHub, ou de um espelho com /MIRROR=<url> (com uma pasta
; v<versão>\ por release, como o AURORA_RELEASE_URL).
;
; O assistente começa pelo idioma (inglês ou português). Os tipos de
; instalação são um por perfil de bundle/components.json (o primeiro é o
; recomendado e padrão), gerados no components.iss com os textos de cada
; idioma, e o Custom, para uso avançado. A lista de aplicativos aparece
; sempre, marcada pelo perfil e travada; só o Custom deixa marcar. Entre ela
; e a caixa dos perfis vai para quem é o perfil (ProfileDetail, também do
; components.iss).
; Reinstalar troca o bundle pelo da nova seleção; os componentes que a pasta
; já tem (do assistente ou do `aurora install`) vêm marcados.
;
; A cara da AURORA: o ícone da AURORA IDE, as imagens de art\ (geradas da
; marca por scripts/installer_art.py) e o modo claro ou escuro do Windows.
;
; A AURORA substitui o Lace. O `lace update` do Lace (0.4 a 0.9) abre este
; assistente, que o release.yml publica também com o nome que ele procura
; (lace-<versão>-windows-x64-setup.exe; crates/aurora-installer/src/lace.rs).
; Com o Lace instalado, o assistente avisa na página de boas-vindas, marca
; os aplicativos da pasta dele e, no fim, depois de pôr a AURORA no PATH,
; roda o desinstalador dele (a tarefa removelace) e espera ele terminar.

#ifndef Stage
  #error Pass /DStage=<aurora-pack inno stage>
#endif
#ifndef AuroraVersion
  #error Pass /DAuroraVersion=<AURORA version>
#endif
#ifndef Bundle
  #define Bundle "?"
#endif
; O AppId da lista de programas instalados. Um outro, só para testar o
; assistente sem mexer na instalação de verdade.
#ifndef AppGuid
  #define AppGuid "B3198760-F391-4D95-BD56-AE9ED09C028E"
#endif
; O AppId do Lace (o installer/windows/lace.iss até a 0.9.0), pelo qual o
; assistente acha o Lace instalado. Um outro, só para testar sem mexer no
; Lace de verdade.
#ifndef LaceGuid
  #define LaceGuid "E34F1510-EC32-4839-8048-14CA03F9EFBB"
#endif
#ifndef ReleaseBase
  #define ReleaseBase "https://github.com/nipscernlab-sandbox/aurora/releases/download/v" + AuroraVersion + "/"
#endif

; [Types], [Components], [Files], o #define DefaultProfile (o nome do perfil
; padrão), SelectInstalledComponents e ProfileDetail.
#include AddBackslash(Stage) + "components.iss"
#ifdef Web
; AddDownloads e SelectedApps.
#include AddBackslash(Stage) + "web.iss"
#endif

[Setup]
AppId={{{#AppGuid}}
AppName=AURORA
AppVersion={#AuroraVersion}
AppVerName=AURORA {#AuroraVersion}
AppPublisher=NIPS-CERN, Faculdade de Engenharia da UFJF
AppPublisherURL=https://nipscern.com
AppComments=NIPS-CERN development tools, from SAPHO processors to Verilog, and AURORA IDE (bundle {#Bundle})
VersionInfoVersion={#AuroraVersion}
DefaultDirName={code:DefaultDir}
DisableProgramGroupPage=yes
DisableWelcomePage=no
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0
ShowLanguageDialog=yes
; Claro ou escuro, como o Windows estiver.
WizardStyle=modern dynamic
; Maior que o padrão (120,120), para a lista de aplicativos caber inteira na
; página de componentes, sem rolar. As imagens de art\ seguem esta proporção
; (SIZE_PERCENT em scripts/installer_art.py).
WizardSizePercent=130,150
SetupIconFile=..\..\assets\brand\icon.ico
UninstallDisplayIcon={app}\toolchain\studio\aurora-ide.exe
; As imagens em 100% e 200%; o assistente escolhe pela escala da tela. São
; as mesmas no modo escuro: o fundo delas já é o azul da marca.
WizardImageFile=art\wizard-164.png,art\wizard-328.png
WizardImageFileDynamicDark=art\wizard-164.png,art\wizard-328.png
WizardSmallImageFile=art\small-55.png,art\small-110.png
WizardSmallImageFileDynamicDark=art\small-55.png,art\small-110.png
; A lista de aplicativos aparece sempre, marcada pelo perfil escolhido: o
; usuário vê exatamente o que vai ser instalado. Com um perfil, a lista fica
; travada; só a instalação Custom deixa marcar (UpdateProfileText).
AlwaysShowComponentsList=yes
ChangesEnvironment=yes
Compression=lzma2/ultra64
SolidCompression=yes
LZMAUseSeparateProcess=yes
UninstallDisplayName=AURORA {#AuroraVersion}
#ifdef Web
OutputBaseFilename=aurora-{#AuroraVersion}-windows-x64-web-setup
#else
OutputBaseFilename=aurora-{#AuroraVersion}-windows-x64-setup
#endif

; O assistente começa perguntando o idioma, com o do Windows já escolhido.
; Os nomes (en, pt) são os códigos das traduções de bundle/components.json,
; que o components.iss usa nas mensagens dos tipos e dos componentes.
[Languages]
Name: "en"; MessagesFile: "compiler:Default.isl"
Name: "pt"; MessagesFile: "compiler:Languages\BrazilianPortuguese.isl"

[Messages]
en.WelcomeLabel1=Welcome to AURORA {#AuroraVersion}
pt.WelcomeLabel1=Boas-vindas à AURORA {#AuroraVersion}
#ifdef Web
en.WelcomeLabel2=AURORA orchestrates the development tools of NIPS-CERN, from SAPHO processors to Verilog, and AURORA IDE is its graphical environment.%n%nThis wizard downloads from the AURORA release only the apps you choose, at the exact versions of bundle {#Bundle}, each one checked against its SHA-256. Verilator comes with the g++, make and Perl it uses, so there is no need to install MSYS2.%n%nNext you choose a profile: {#DefaultProfile} is the recommended one, and the Custom installation is for advanced use.
pt.WelcomeLabel2=A AURORA orquestra as ferramentas de desenvolvimento do NIPS-CERN, dos processadores SAPHO ao Verilog, e a AURORA IDE é o ambiente gráfico dela.%n%nEste assistente baixa da release da AURORA só os aplicativos que você escolher, nas versões exatas do bundle {#Bundle}, cada um conferido pelo SHA-256. O Verilator vem com o g++, o make e o Perl que ele usa: não é preciso instalar o MSYS2.%n%nEm seguida você escolhe um perfil: o {#DefaultProfile} é o recomendado, e a instalação personalizada é para uso avançado.
#else
en.WelcomeLabel2=AURORA orchestrates the development tools of NIPS-CERN, from SAPHO processors to Verilog, and AURORA IDE is its graphical environment.%n%nAURORA only uses the tools of the bundle installed with it, at the exact versions of bundle {#Bundle}. Verilator comes with the g++, make and Perl it uses, so there is no need to install MSYS2.%n%nNext you choose a profile: {#DefaultProfile} is the recommended one, and the Custom installation is for advanced use.
pt.WelcomeLabel2=A AURORA orquestra as ferramentas de desenvolvimento do NIPS-CERN, dos processadores SAPHO ao Verilog, e a AURORA IDE é o ambiente gráfico dela.%n%nA AURORA só usa as ferramentas do bundle instalado com ele, nas versões exatas do bundle {#Bundle}. O Verilator vem com o g++, o make e o Perl que ele usa: não é preciso instalar o MSYS2.%n%nEm seguida você escolhe um perfil: o {#DefaultProfile} é o recomendado, e a instalação personalizada é para uso avançado.
#endif
en.WizardSelectComponents=Choose a profile
pt.WizardSelectComponents=Escolha um perfil
en.SelectComponentsDesc=Which apps should be installed?
pt.SelectComponentsDesc=Quais aplicativos serão instalados?
en.SelectComponentsLabel2=Each profile installs the apps marked in the list. To choose them yourself, pick Custom (advanced). To add apps later: aurora install.
pt.SelectComponentsLabel2=Cada perfil instala os aplicativos marcados na lista. Para escolher você mesmo, use a Personalizada (avançada). Para acrescentar aplicativos depois: aurora install.
en.FinishedHeadingLabel=AURORA is ready
pt.FinishedHeadingLabel=A AURORA está pronta
en.FinishedLabel=Open AURORA IDE from the Start menu, or open a new terminal and run: aurora tools --verify
pt.FinishedLabel=Abra a AURORA IDE pelo menu Iniciar, ou abra um terminal novo e rode: aurora tools --verify

; Os textos do [Code], das tarefas e do [Run]. Os dos tipos e dos
; componentes vêm do components.iss.
[CustomMessages]
en.TaskPath=Add AURORA to PATH, to use "aurora" in any terminal
pt.TaskPath=Pôr a AURORA no PATH, para usar o "aurora" em qualquer terminal
en.TaskStudioIcon=Create a desktop shortcut for AURORA IDE
pt.TaskStudioIcon=Criar um atalho da AURORA IDE na área de trabalho
en.RunStudio=Open AURORA IDE
pt.RunStudio=Abrir a AURORA IDE
en.PreviousInstall=AURORA %1 is installed on this computer: this wizard replaces it with AURORA %2, and the apps it has stay selected.
pt.PreviousInstall=A AURORA %1 está instalada neste computador: este assistente a troca pela AURORA %2, e os aplicativos que ela tem continuam marcados.
en.DownloadTitle=Downloading the apps
pt.DownloadTitle=Baixando os aplicativos
en.DownloadDesc=AURORA %1: the chosen apps, each checked against its SHA-256.
pt.DownloadDesc=AURORA %1: os aplicativos escolhidos, cada um conferido pelo SHA-256.
en.ExtractTitle=Installing the apps
pt.ExtractTitle=Instalando os aplicativos
en.ExtractDesc=Extracting the downloaded apps into the installation folder.
pt.ExtractDesc=Extraindo os aplicativos baixados na pasta da instalação.
en.DownloadFailed=Could not download %1: %2
pt.DownloadFailed=Não foi possível baixar %1: %2
en.SetupRunFailed=Could not run the aurora program that installs the apps: %1
pt.SetupRunFailed=Não foi possível rodar o aurora que instala os aplicativos: %1
en.SetupFailed=The apps could not be installed. %1
pt.SetupFailed=Não foi possível instalar os aplicativos. %1
en.SpaceInPath=The selected folder has a space in its path, and the make used by Verilator does not accept spaces.
pt.SpaceInPath=A pasta escolhida tem um espaço no caminho, e o make que o Verilator usa não aceita espaços.
en.InstallThereAnyway=Install there anyway?
pt.InstallThereAnyway=Instalar lá mesmo assim?
en.WebView2Missing=AURORA IDE uses the Microsoft Edge WebView2 Runtime, which is not installed on this computer. AURORA IDE will open only after you install it from https://developer.microsoft.com/microsoft-edge/webview2/ (Evergreen Runtime); the rest of AURORA works without it.
pt.WebView2Missing=A AURORA IDE usa o Microsoft Edge WebView2 Runtime, que não está instalado neste computador. A AURORA IDE só abre depois que você o instalar de https://developer.microsoft.com/microsoft-edge/webview2/ (Evergreen Runtime); o resto da AURORA funciona sem ele.
en.InstallNowAnyway=Install AURORA now anyway?
pt.InstallNowAnyway=Instalar a AURORA agora mesmo assim?
en.ForeignToolchain=The folder %1 already exists and is not from an AURORA installation, and installing would replace it. Choose another folder, or move it away first.
pt.ForeignToolchain=A pasta %1 já existe e não é de uma instalação da AURORA, e instalar a substituiria. Escolha outra pasta, ou tire-a de lá antes.
en.LaceInstalled=Lace %1 is installed in %2. AURORA replaces it: the apps it has come selected, and at the end this wizard uninstalls it. Projects and settings stay.
pt.LaceInstalled=O Lace %1 está instalado em %2. A AURORA o substitui: os aplicativos dele vêm marcados, e no fim este assistente o desinstala. Os projetos e as preferências ficam.
en.TaskRemoveLace=Uninstall Lace, which AURORA replaces
pt.TaskRemoveLace=Desinstalar o Lace, que a AURORA substitui
en.RemoveLaceTitle=Uninstalling Lace
pt.RemoveLaceTitle=Desinstalando o Lace
en.RemoveLaceDesc=AURORA is installed. The Lace %1 uninstaller is removing %2, which can take a few minutes.
pt.RemoveLaceDesc=A AURORA já está instalada. O desinstalador do Lace %1 está apagando %2, o que pode levar alguns minutos.

[Tasks]
Name: "path"; Description: "{cm:TaskPath}"
Name: "studioicon"; Description: "{cm:TaskStudioIcon}"; Components: studio; Flags: unchecked
Name: "removelace"; Description: "{cm:TaskRemoveLace}"; Check: LaceInstalled

; A AURORA IDE fica no bundle (toolchain\studio, o componente studio) e acha
; o bundle por estar dentro dele.
[Icons]
Name: "{autoprograms}\AURORA IDE"; Filename: "{app}\toolchain\studio\aurora-ide.exe"; Components: studio
Name: "{autodesktop}\AURORA IDE"; Filename: "{app}\toolchain\studio\aurora-ide.exe"; Components: studio; Tasks: studioicon

; A AURORA IDE que pediu a atualização (`aurora update --reopen-ide`, que
; passa /REOPENIDE=1) fechou para o assistente trocar os arquivos dele: ele
; abre de novo sozinho, sem a caixa da página final. O Lace Studio, que
; atualiza pelo `lace update` e passa /REOPENSTUDIO=1, também fechou: no
; lugar dele abre a AURORA IDE.
[Run]
Filename: "{app}\toolchain\studio\aurora-ide.exe"; Components: studio; Flags: nowait runasoriginaluser; Check: ReopenIde
Filename: "{app}\toolchain\studio\aurora-ide.exe"; Description: "{cm:RunStudio}"; Components: studio; Flags: postinstall nowait skipifsilent unchecked; Check: not ReopenIde

[InstallDelete]
; Os atalhos de quando a AURORA IDE se chamava Lace Studio: os de agora têm
; o nome novo, e os velhos ficariam ao lado deles.
Type: files; Name: "{autoprograms}\Lace Studio.lnk"
Type: files; Name: "{autodesktop}\Lace Studio.lnk"
#ifndef Web
; Reinstalar troca o bundle inteiro: o que foi desmarcado sai. O assistente
; que baixa os aplicativos troca o toolchain\ ele mesmo (o `aurora setup`).
Type: filesandordirs; Name: "{app}\toolchain"
#endif
; As pastas .antigo-* e .instalando-* são sobras do aurora update por
; componentes (o aurora.exe que rodava a atualização fica numa .antigo-*).
; O diário de uma atualização que parou no meio também sai: o assistente
; troca tudo, e ele não vale mais (crates/aurora-installer/src/update.rs).
Type: filesandordirs; Name: "{app}\.antigo-*"
Type: filesandordirs; Name: "{app}\.instalando-*"
Type: files; Name: "{app}\.atualizando.json"
Type: files; Name: "{app}\.atualizando.json.tmp"

[UninstallDelete]
Type: filesandordirs; Name: "{app}\toolchain"
Type: filesandordirs; Name: "{app}\.antigo-*"
Type: filesandordirs; Name: "{app}\.instalando-*"
Type: files; Name: "{app}\.atualizando.json"
Type: files; Name: "{app}\.atualizando.json.tmp"
; A trava das instalações e atualizações do aurora.
Type: files; Name: "{app}\.instalacao.lock"

[Code]
const
  UserEnvKey = 'Environment';
  SystemEnvKey = 'SYSTEM\CurrentControlSet\Control\Session Manager\Environment';
  { O cliente do WebView2 no EdgeUpdate (ver HasWebView2). }
  WebView2Client = '\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}';
  { A entrada da AURORA na lista de programas instalados. }
  UninstallKey = 'Software\Microsoft\Windows\CurrentVersion\Uninstall\{{#AppGuid}}_is1';
  { A do Lace, que a AURORA substitui. }
  LaceUninstallKey = 'Software\Microsoft\Windows\CurrentVersion\Uninstall\{{#LaceGuid}}_is1';
  { Quanto esperar o desinstalador do Lace terminar, em milissegundos: apagar
    o bundle dele leva minutos. }
  LaceUninstallWait = 600000;

var
  Preselected: Boolean;
  { Na página de componentes, entre a caixa dos perfis e a lista: para quem
    é o perfil escolhido. }
  ProfileText: TNewStaticText;
  PreviousTypesChange: TNotifyEvent;
  PreviousClickCheck: TNotifyEvent;
  { Enquanto o desinstalador do Lace roda (RemoveLace). }
  RemoveLacePage: TOutputProgressWizardPage;
#ifdef Web
  DownloadPage: TDownloadWizardPage;
  ExtractPage: TOutputProgressWizardPage;
  { A última linha do `aurora setup`, para a mensagem de erro. }
  SetupLine: String;
#endif

{ Pasta padrão sem espaço no caminho: o make do Verilator não aceita
  espaços, e "Program Files" tem um. }
function DefaultDir(Param: String): String;
begin
  if IsAdminInstallMode then
    Result := ExpandConstant('{sd}\AURORA')
  else
    Result := ExpandConstant('{localappdata}\Programs\AURORA');
end;

{ O icacls do Windows em Dir, com Params; uma falha fica no log. }
procedure Icacls(Dir, Params: String);
var
  ResultCode: Integer;
begin
  if not Exec(ExpandConstant('{sys}\icacls.exe'), '"' + Dir + '" ' + Params, '', SW_HIDE, ewWaitUntilTerminated, ResultCode) or (ResultCode <> 0) then
    Log(Format('icacls %s %s exited with %d', [Dir, Params, ResultCode]));
end;

{ No modo de administrador, a pasta da instalação (C:\AURORA) herdaria de
  C:\ o Modify de "Usuários autenticados": qualquer usuário da máquina
  trocaria o aurora.exe, que fica no PATH do sistema, ou plantaria uma DLL ao
  lado dele, e o próximo administrador a rodaria. A pasta fica só com as
  permissões dela, para tudo que ela tem e vier a ter: controle total para
  os administradores e o sistema, e leitura e execução para os usuários.
  Antes, os administradores viram donos de tudo (um dono pode devolver a si
  mesmo o que a ACL tira), e a ACL de cada arquivo volta a ser só a herdada,
  sem as permissões que alguém pôs à mão (quem criou a pasta antes da
  instalação, por exemplo). Pelos SIDs, porque os nomes dos grupos mudam com
  o idioma do Windows. Na instalação só do usuário, a pasta é dele e fica
  como está. }
procedure SecureAdminFolder;
var
  Dir: String;
begin
  if not IsAdminInstallMode then
    exit;
  Dir := ExpandConstant('{app}');
  if not ForceDirectories(Dir) then begin
    Log('Could not create ' + Dir + ' to set its permissions');
    exit;
  end;
  Icacls(Dir, '/setowner *S-1-5-32-544 /T /C /Q');
  Icacls(Dir, '/reset /T /C /Q');
  Icacls(Dir, '/inheritance:r /grant:r *S-1-5-32-544:(OI)(CI)F *S-1-5-18:(OI)(CI)F *S-1-5-32-545:(OI)(CI)RX /C /Q');
end;

{ A pasta tem uma instalação da AURORA: o cabeçalho do bundle ou o aurora.exe. }
function IsAuroraFolder(Dir: String): Boolean;
begin
  Result := FileExists(AddBackslash(Dir) + 'toolchain\bundle.json') or FileExists(AddBackslash(Dir) + 'bin\aurora.exe');
end;

function EnvRoot: Integer;
begin
  if IsAdminInstallMode then
    Result := HKEY_LOCAL_MACHINE
  else
    Result := HKEY_CURRENT_USER;
end;

function EnvKey: String;
begin
  if IsAdminInstallMode then
    Result := SystemEnvKey
  else
    Result := UserEnvKey;
end;

function PathHas(Paths, Dir: String): Boolean;
begin
  Result := Pos(';' + Uppercase(Dir) + ';', ';' + Uppercase(Paths) + ';') > 0;
end;

procedure AddToPath(Dir: String);
var
  Paths: String;
begin
  if not RegQueryStringValue(EnvRoot, EnvKey, 'Path', Paths) then
    Paths := '';
  if PathHas(Paths, Dir) then
    exit;
  if (Paths <> '') and (Paths[Length(Paths)] <> ';') then
    Paths := Paths + ';';
  RegWriteExpandStringValue(EnvRoot, EnvKey, 'Path', Paths + Dir);
end;

procedure RemoveFromPath(Dir: String);
var
  Paths: String;
  P: Integer;
begin
  if not RegQueryStringValue(EnvRoot, EnvKey, 'Path', Paths) then
    exit;
  Paths := ';' + Paths + ';';
  P := Pos(';' + Uppercase(Dir) + ';', Uppercase(Paths));
  if P = 0 then
    exit;
  Delete(Paths, P, Length(Dir) + 1);
  Paths := Copy(Paths, 2, Length(Paths) - 2);
  if Paths = '' then
    RegDeleteValue(EnvRoot, EnvKey, 'Path')
  else
    RegWriteExpandStringValue(EnvRoot, EnvKey, 'Path', Paths);
end;

{ A AURORA IDE usa o WebView2, que vem com o Windows 11 e com o Windows 10
  atualizado. A Microsoft manda conferir a versão ("pv") nestas chaves. }
function HasWebView2: Boolean;
var
  Version: String;
begin
  Result :=
    (RegQueryStringValue(HKLM, 'SOFTWARE\WOW6432Node' + WebView2Client, 'pv', Version) and (Version <> '') and (Version <> '0.0.0.0')) or
    (RegQueryStringValue(HKLM, 'SOFTWARE' + WebView2Client, 'pv', Version) and (Version <> '') and (Version <> '0.0.0.0')) or
    (RegQueryStringValue(HKCU, 'Software' + WebView2Client, 'pv', Version) and (Version <> '') and (Version <> '0.0.0.0'));
end;

{ O Lace instalado pelo assistente dele: a entrada na lista de programas,
  do usuário ou de todos, com a pasta e a versão. }
function FindLace(var Root: Integer; var Dir, Version: String): Boolean;
begin
  Result := False;
  if RegQueryStringValue(HKCU, LaceUninstallKey, 'InstallLocation', Dir) then
    Root := HKCU
  else if RegQueryStringValue(HKLM, LaceUninstallKey, 'InstallLocation', Dir) then
    Root := HKLM
  else
    exit;
  if not RegQueryStringValue(Root, LaceUninstallKey, 'DisplayVersion', Version) then
    Version := '';
  Dir := RemoveBackslashUnlessRoot(Dir);
  Result := Dir <> '';
end;

function LaceInstalled: Boolean;
var
  Root: Integer;
  Dir, Version: String;
begin
  Result := FindLace(Root, Dir, Version);
end;

{ Tira o Lace pelo desinstalador dele, sem perguntas, e espera ele terminar.
  O desinstalador do Inno Setup roda uma cópia de si na pasta temporária, e
  o ShellExec volta quando ela termina, já sem a entrada do Lace na lista de
  programas; esperar a entrada sair é uma segurança. Apagar o bundle do Lace
  leva minutos, e a página de progresso mostra o que está acontecendo. Pelo
  ShellExec, que pede o administrador quando o Lace foi instalado para
  todos. Se não der, o Lace fica, e o log diz por quê. }
procedure RemoveLace;
var
  Root, ResultCode, Waited: Integer;
  Dir, Version, Uninstaller: String;
begin
  if not FindLace(Root, Dir, Version) then
    exit;
  if not RegQueryStringValue(Root, LaceUninstallKey, 'UninstallString', Uninstaller) then begin
    Log('Lace ' + Version + ' in ' + Dir + ' has no UninstallString; it stays');
    exit;
  end;
  Uninstaller := RemoveQuotes(Uninstaller);
  Log('Uninstalling Lace ' + Version + ' from ' + Dir + ' with ' + Uninstaller);
  RemoveLacePage.SetText(FmtMessage(CustomMessage('RemoveLaceDesc'), [Version, Dir]), '');
  RemoveLacePage.ProgressBar.Style := npbstMarquee;
  RemoveLacePage.Show;
  try
    if not ShellExec('', Uninstaller, '/VERYSILENT /SUPPRESSMSGBOXES /NORESTART', '', SW_SHOWNORMAL, ewWaitUntilTerminated, ResultCode) then begin
      Log('Could not run the Lace uninstaller: ' + SysErrorMessage(ResultCode));
      exit;
    end;
    Waited := 0;
    while RegKeyExists(Root, LaceUninstallKey) and (Waited < LaceUninstallWait) do begin
      { O SetProgress também processa as mensagens da janela. }
      RemoveLacePage.SetProgress(0, 1);
      Sleep(250);
      Waited := Waited + 250;
    end;
  finally
    RemoveLacePage.Hide;
  end;
  if RegKeyExists(Root, LaceUninstallKey) then
    Log('Lace is still installed after waiting ' + IntToStr(Waited div 1000) + ' s for its uninstaller (exit code ' + IntToStr(ResultCode) + ')')
  else
    Log('Lace was uninstalled after ' + IntToStr(Waited div 1000) + ' s');
end;

{ /REOPENIDE=1: o `aurora update` da AURORA IDE abriu este assistente, e ela
  fechou para ele (ver [Run]). /REOPENSTUDIO=1 é o mesmo pedido do Lace
  Studio, pelo `lace update`. }
function ReopenIde: Boolean;
begin
  Result := (ExpandConstant('{param:REOPENIDE|0}') = '1') or (ExpandConstant('{param:REOPENSTUDIO|0}') = '1');
end;

{ Os componentes que a pasta escolhida já tem vêm marcados, uma vez: na
  página de componentes, ou antes de instalar numa instalação silenciosa
  sem /COMPONENTS. Numa pasta sem a AURORA, com o Lace instalado, os que a
  pasta do Lace tem. Uma seleção que não é a de nenhum perfil põe o tipo em
  Custom, com a lista. }
procedure PreselectOnce;
var
  Root: Integer;
  Dir, Version: String;
begin
  if not Preselected then begin
    Preselected := True;
    if not IsAuroraFolder(WizardDirValue) and FindLace(Root, Dir, Version) then
      SelectInstalledComponents(Dir)
    else
      SelectInstalledComponents(WizardDirValue);
  end;
end;

#ifdef Web
{ De onde vêm os arquivos da release: o GitHub, ou o espelho de /MIRROR. }
function ReleaseBase: String;
var
  Mirror: String;
begin
  Mirror := ExpandConstant('{param:MIRROR|}');
  if Mirror = '' then
    Result := '{#ReleaseBase}'
  else begin
    if Copy(Mirror, Length(Mirror), 1) <> '/' then
      Mirror := Mirror + '/';
    Result := Mirror + 'v{#AuroraVersion}/';
  end;
end;

{ Baixa para a pasta temporária o índice e os pedaços dos componentes marcados,
  cada um conferido pelo SHA-256 que o aurora-pack gravou no web.iss. }
function DownloadApps: Boolean;
begin
  DownloadPage.Clear;
  AddDownloads(DownloadPage, ReleaseBase);
  DownloadPage.Show;
  try
    try
      DownloadPage.Download;
      Result := True;
    except
      if DownloadPage.AbortedByUser then
        Log('The download was cancelled.')
      else
        SuppressibleMsgBox(AddPeriod(FmtMessage(CustomMessage('DownloadFailed'), [DownloadPage.LastBaseNameOrUrl, GetExceptionMessage])), mbCriticalError, MB_OK, IDOK);
      Result := False;
    end;
  finally
    DownloadPage.Hide;
  end;
end;

{ Cada linha do `aurora setup`: o pedaço que ele está extraindo, "[2/5] Yosys",
  vira o texto e a barra da página de progresso. }
procedure OnSetupOutput(const S: String; const Error, FirstLine: Boolean);
var
  Line, Counter: String;
  Close, Slash, Done, Total: Integer;
begin
  Line := Trim(S);
  if Line = '' then
    exit;
  Log('aurora setup: ' + Line);
  SetupLine := Line;
  Close := Pos(']', Line);
  if (Copy(Line, 1, 1) = '[') and (Close > 0) then begin
    Counter := Copy(Line, 2, Close - 2);
    Slash := Pos('/', Counter);
    if Slash > 0 then begin
      Done := StrToIntDef(Copy(Counter, 1, Slash - 1), 0);
      Total := StrToIntDef(Copy(Counter, Slash + 1, Length(Counter)), 0);
      if (Total > 0) and (Done > 0) then
        ExtractPage.SetProgress(Done - 1, Total);
    end;
  end;
  ExtractPage.SetText(CustomMessage('ExtractTitle'), Line);
end;

{ Antes de copiar o aurora.exe: o `aurora setup` extrai os pedaços baixados,
  confere os executáveis e troca o toolchain\ da pasta. Se ele falha, o
  assistente para sem ter copiado nada. }
function PrepareToInstall(var NeedsRestart: Boolean): String;
var
  Params, Apps: String;
  ResultCode: Integer;
begin
  Result := '';
  { Antes do `aurora setup`, que já grava o toolchain\ na pasta. }
  SecureAdminFolder;
  ExtractTemporaryFile('aurora.exe');
  Params := 'setup --payload "' + ExpandConstant('{tmp}') + '" --prefix "' + WizardDirValue + '"';
  Apps := SelectedApps;
  if Apps <> '' then
    Params := Params + ' --components ' + Apps;
  ExtractPage.SetText(CustomMessage('ExtractTitle'), '');
  ExtractPage.SetProgress(0, 1);
  ExtractPage.Show;
  try
    SetupLine := '';
    if not ExecAndLogOutput(ExpandConstant('{tmp}\aurora.exe'), Params, ExpandConstant('{tmp}'), SW_HIDE, ewWaitUntilTerminated, ResultCode, @OnSetupOutput) then
      Result := FmtMessage(CustomMessage('SetupRunFailed'), [SysErrorMessage(ResultCode)])
    else if ResultCode <> 0 then
      Result := FmtMessage(CustomMessage('SetupFailed'), [SetupLine]);
  finally
    ExtractPage.Hide;
  end;
end;
#endif

{ O texto do perfil escolhido, e a lista travada: com um perfil, ela só
  mostra o que vai ser instalado; só a instalação Custom deixa marcar. }
procedure UpdateProfileText;
begin
  ProfileText.Caption := ProfileDetail(WizardSetupType(False));
  WizardForm.ComponentsList.Enabled := WizardSetupType(False) = 'avancada';
end;

{ Trocar o perfil na caixa: o assistente marca os aplicativos dele na lista
  (o tratamento original) e o texto muda junto. }
procedure TypesComboChange(Sender: TObject);
begin
  PreviousTypesChange(Sender);
  UpdateProfileText;
end;

{ Marcar ou desmarcar um aplicativo à mão (só no Custom, com a lista
  destravada): o tratamento original, e o texto acompanha. }
procedure ComponentsClickCheck(Sender: TObject);
begin
  PreviousClickCheck(Sender);
  UpdateProfileText;
end;

procedure InitializeWizard;
var
  Previous, LaceDir, LaceVersion: String;
  Shift, LaceRoot: Integer;
begin
  { A linha do perfil entra embaixo da caixa, e a lista desce para dar lugar. }
  ProfileText := TNewStaticText.Create(WizardForm);
  ProfileText.Parent := WizardForm.SelectComponentsPage;
  ProfileText.AutoSize := False;
  ProfileText.WordWrap := True;
  { O & de "CLI & Core" é texto, não atalho de teclado. }
  ProfileText.ShowAccelChar := False;
  ProfileText.Left := WizardForm.ComponentsList.Left;
  ProfileText.Width := WizardForm.ComponentsList.Width;
  ProfileText.Top := WizardForm.TypesCombo.Top + WizardForm.TypesCombo.Height + ScaleY(6);
  ProfileText.Height := ScaleY(58);
  Shift := ProfileText.Top + ProfileText.Height + ScaleY(4) - WizardForm.ComponentsList.Top;
  WizardForm.ComponentsList.Top := WizardForm.ComponentsList.Top + Shift;
  WizardForm.ComponentsList.Height := WizardForm.ComponentsList.Height - Shift;
  PreviousTypesChange := WizardForm.TypesCombo.OnChange;
  WizardForm.TypesCombo.OnChange := @TypesComboChange;
  PreviousClickCheck := WizardForm.ComponentsList.OnClickCheck;
  WizardForm.ComponentsList.OnClickCheck := @ComponentsClickCheck;

  if RegQueryStringValue(HKA, UninstallKey, 'DisplayVersion', Previous) and (Previous <> '') then
    WizardForm.WelcomeLabel2.Caption := WizardForm.WelcomeLabel2.Caption + #13#10#13#10 +
      FmtMessage(CustomMessage('PreviousInstall'), [Previous, '{#AuroraVersion}']);
  if FindLace(LaceRoot, LaceDir, LaceVersion) then
    WizardForm.WelcomeLabel2.Caption := WizardForm.WelcomeLabel2.Caption + #13#10#13#10 +
      FmtMessage(CustomMessage('LaceInstalled'), [LaceVersion, LaceDir]);
  RemoveLacePage := CreateOutputProgressPage(CustomMessage('RemoveLaceTitle'), '');
#ifdef Web
  DownloadPage := CreateDownloadPage(CustomMessage('DownloadTitle'), FmtMessage(CustomMessage('DownloadDesc'), ['{#AuroraVersion}']), nil);
  DownloadPage.ShowBaseNameInsteadOfUrl := True;
  ExtractPage := CreateOutputProgressPage(CustomMessage('ExtractTitle'), CustomMessage('ExtractDesc'));
#endif
end;

procedure CurPageChanged(CurPageID: Integer);
begin
  if CurPageID = wpSelectComponents then begin
    PreselectOnce;
    UpdateProfileText;
  end;
end;

{ Os avisos só aparecem na instalação com o assistente; na silenciosa
  (/VERYSILENT) um MsgBox esperaria um clique para sempre, então vão só para
  o log. }
function NextButtonClick(CurPageID: Integer): Boolean;
begin
  Result := True;
  { Uma toolchain\ de outra ferramenta na pasta escolhida iria embora
    inteira, sem passar pela lixeira: o assistente completo a troca
    ([InstallDelete]), e o que baixa os aplicativos também (o `aurora
    setup`, que recusa e pararia só no fim). }
  if (CurPageID = wpSelectDir) and DirExists(AddBackslash(WizardDirValue) + 'toolchain') and not IsAuroraFolder(WizardDirValue) then begin
    Log('Error: ' + AddBackslash(WizardDirValue) + 'toolchain exists and is not from an AURORA installation.');
    if not WizardSilent then
      MsgBox(FmtMessage(CustomMessage('ForeignToolchain'), [AddBackslash(WizardDirValue) + 'toolchain']), mbError, MB_OK);
    Result := False;
    exit;
  end;
  if (CurPageID = wpSelectDir) and (Pos(' ', WizardDirValue) > 0) then begin
    Log('Warning: The folder path has a space, and the make used by Verilator does not accept spaces.');
    if not WizardSilent then
      Result := MsgBox(CustomMessage('SpaceInPath') + #13#10#13#10 + CustomMessage('InstallThereAnyway'), mbConfirmation, MB_YESNO) = IDYES;
  end;
  if CurPageID = wpReady then begin
    if WizardSilent and (ExpandConstant('{param:COMPONENTS|}') = '') then
      PreselectOnce;
    { Antes de instalar, e não depois: sem o WebView2, o Studio não abre. }
    if WizardIsComponentSelected('studio') and not HasWebView2 then begin
      Log('Warning: The WebView2 Runtime, which AURORA IDE uses, is not installed.');
      if not WizardSilent then
        Result := MsgBox(CustomMessage('WebView2Missing') + #13#10#13#10 + CustomMessage('InstallNowAnyway'), mbConfirmation, MB_YESNO) = IDYES;
    end;
#ifdef Web
    if Result then
      Result := DownloadApps;
#endif
  end;
end;

procedure CurStepChanged(CurStep: TSetupStep);
begin
  { Antes de copiar os arquivos, que herdam as permissões da pasta. }
  if CurStep = ssInstall then
    SecureAdminFolder;
  if CurStep = ssPostInstall then begin
    { A AURORA no PATH antes de o Lace sair. O desinstalador do Lace tira a
      pasta dele do mesmo valor, e só começa depois desta escrita: ele lê o
      PATH já com a AURORA. E, se o assistente for fechado durante a espera
      pelo Lace, a AURORA já está no PATH. }
    if WizardIsTaskSelected('path') then
      AddToPath(ExpandConstant('{app}\bin'));
    if WizardIsTaskSelected('removelace') then
      RemoveLace;
  end;
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
begin
  if CurUninstallStep = usPostUninstall then
    RemoveFromPath(ExpandConstant('{app}\bin'));
end;
