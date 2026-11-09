# Instala a última release da AURORA no Windows 10 e 11 (x64).
#
#   irm https://raw.githubusercontent.com/nipscernlab-sandbox/aurora/main/install.ps1 | iex
#
# Baixa o assistente de instalação da release, confere o SHA-256 com o
# SHA256SUMS da release e o abre. Desde a 0.7.0, o assistente é o web, que
# baixa só os aplicativos escolhidos. Variáveis opcionais:
#   $env:AURORA_VERSION = "0.2.0"            versão fixa, em vez da última
#   $env:AURORA_SETUP_ARGS = "/VERYSILENT /SUPPRESSMSGBOXES /CURRENTUSER /TYPE=sapho /TASKS=path"
#                                          instalação sem perguntas
#   $env:AURORA_FULL_SETUP = "1"             o assistente completo, com o bundle
#                                          inteiro dentro (para instalar sem rede depois)

& {
    $ErrorActionPreference = 'Stop'
    $ProgressPreference = 'SilentlyContinue'   # a barra do Invoke-WebRequest deixa o download lento no PowerShell 5.1
    # O PowerShell 5.1 do Windows 10 pode não ligar o TLS 1.2, que o GitHub exige.
    [Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12

    # Uma linha de progresso que cabe em $Columns - 1 colunas: uma linha
    # quebrada não volta ao começo com o `r. Vêm a barra e a porcentagem e,
    # enquanto couber, o que já veio, a velocidade média e o tempo que falta;
    # a barra cresce de 10 até 30 colunas com o que sobra. Sem o tamanho,
    # só o que já veio e a velocidade.
    function Format-Progress([long]$Done, [long]$Total, [double]$Seconds, [int]$Columns) {
        $culture = [Globalization.CultureInfo]::InvariantCulture
        $mib = 1048576.0
        $speed = if ($Seconds -gt 0) { $Done / $Seconds } else { 0 }
        $rate = [string]::Format($culture, '  {0:0.0} MiB/s', $speed / $mib)
        if ($Total -le 0) {
            return [string]::Format($culture, '  {0:0.0} MiB', $Done / $mib) + $rate
        }
        $fraction = [Math]::Min(1.0, $Done / [double]$Total)
        $left = ''
        if ($speed -gt 0 -and $Done -lt $Total) {
            $s = [int][Math]::Ceiling(($Total - $Done) / $speed)
            $left = '  {0}:{1:00} left' -f [int][Math]::Floor($s / 60), ($s % 60)
        }
        # Cada parte reserva a largura maior que pode ter, para a barra não
        # mudar de tamanho quando o tempo que falta aparece ou some.
        $parts = @(
            @([string]::Format($culture, '  {0:0.0}/{1:0.0} MiB', $Done / $mib, $Total / $mib),
              [string]::Format($culture, '  {0:0.0}/{0:0.0} MiB', $Total / $mib).Length),
            @($rate, 13),
            @($left, 13))
        $room = $Columns - 1 - 19   # 19: os dois espaços, a barra de 10 entre [] e " 100%"
        $tail = ''
        foreach ($part in $parts) {
            if ($part[1] -gt $room) { break }
            $tail += $part[0]
            $room -= $part[1]
        }
        $bar = 10 + [Math]::Max(0, [Math]::Min(20, $room))
        $filled = [int][Math]::Floor($fraction * $bar)
        return '  [{0}{1}] {2,3}%{3}' -f ('#' * $filled), ('.' * ($bar - $filled)), [int][Math]::Floor($fraction * 100), $tail
    }

    # As colunas da janela; sem janela (a saída vai para um arquivo), sem limite.
    function Get-Columns {
        if ([Console]::IsOutputRedirected) { return 1000 }
        try { $c = $Host.UI.RawUI.WindowSize.Width } catch { $c = 0 }
        if ($c -gt 0) { return $c } else { return 80 }
    }

    # Baixa $Uri em $Path com uma linha de progresso. A barra do
    # Invoke-WebRequest fica desligada porque, no PowerShell 5.1, ela redesenha
    # a cada bloco e deixa o download várias vezes mais lento; aqui o download
    # vai pelo mesmo HttpWebRequest (e pelo mesmo proxy do sistema) e a linha
    # é redesenhada no máximo quatro vezes por segundo.
    function Get-WithProgress([string]$Uri, [string]$Path) {
        $interactive = -not [Console]::IsOutputRedirected
        $request = [Net.WebRequest]::Create($Uri)
        $request.UserAgent = 'aurora-install.ps1'
        $response = $request.GetResponse()
        $width = 0
        try {
            $total = $response.ContentLength
            $in = $response.GetResponseStream()
            $out = [IO.File]::Create($Path)
            try {
                $buffer = New-Object byte[] 262144
                $done = [long]0
                $clock = [Diagnostics.Stopwatch]::StartNew()
                $drawn = [long]-1000
                while (($n = $in.Read($buffer, 0, $buffer.Length)) -gt 0) {
                    $out.Write($buffer, 0, $n)
                    $done += $n
                    if ($interactive -and $clock.ElapsedMilliseconds - $drawn -ge 250) {
                        $drawn = $clock.ElapsedMilliseconds
                        # A janela pode mudar de largura no meio do download.
                        $columns = Get-Columns
                        $line = Format-Progress $done $total $clock.Elapsed.TotalSeconds $columns
                        Write-Host -NoNewline ("`r" + $line.PadRight([Math]::Min($width, $columns - 1)))
                        $width = $line.Length
                    }
                }
            } finally {
                $out.Dispose()
                $in.Dispose()
            }
        } finally {
            $response.Dispose()
        }
        $columns = Get-Columns
        $line = Format-Progress $done $total $clock.Elapsed.TotalSeconds $columns
        if ($interactive) { Write-Host ("`r" + $line.PadRight([Math]::Min($width, $columns - 1))) } else { Write-Host $line }
        if ($total -gt 0 -and $done -ne $total) {
            throw "The download stopped at $done of $total bytes"
        }
    }

    $repo = if ($env:AURORA_REPO) { $env:AURORA_REPO } else { 'nipscernlab-sandbox/aurora' }
    $version = $env:AURORA_VERSION
    if (-not $version) {
        # A página da última release redireciona para /releases/tag/<tag>,
        # e é o que a CLI consulta: a API do GitHub limita a 60 consultas por
        # hora por IP, e uma turma num laboratório atrás de um IP só passava
        # disso no meio da aula. O endereço final vem no ResponseUri no
        # PowerShell 5.1 e no RequestMessage no 7.
        $latest = Invoke-WebRequest -UseBasicParsing -Method Head -Uri "https://github.com/$repo/releases/latest"
        $final = if ($latest.BaseResponse.ResponseUri) { $latest.BaseResponse.ResponseUri } else { $latest.BaseResponse.RequestMessage.RequestUri }
        $tag = ([string]$final.AbsoluteUri -split '/releases/tag/')[-1]
        if ($tag -notmatch '^v?[0-9]+\.[0-9]+\.[0-9]+[0-9A-Za-z.+-]*$') {
            throw "Could not find the latest release of $repo ($final)"
        }
        $version = $tag.TrimStart('v')
    }

    $base = "https://github.com/$repo/releases/download/v$version"
    $dir = Join-Path ([IO.Path]::GetTempPath()) "aurora-$version"
    New-Item -ItemType Directory -Force -Path $dir | Out-Null
    $sums = Join-Path $dir 'SHA256SUMS'
    Invoke-WebRequest -UseBasicParsing -Uri "$base/SHA256SUMS" -OutFile $sums
    $listed = Get-Content $sums
    $listedAs = { param($file) $listed | Where-Object { $_ -match ('\s\*?' + [regex]::Escape($file) + '$') } | Select-Object -First 1 }

    # O assistente web (desde a 0.7.0) tem poucos MB e baixa da release só
    # os aplicativos escolhidos; o completo traz o bundle inteiro, e é o que
    # vem numa versão sem o web ou com AURORA_FULL_SETUP.
    $name = "aurora-$version-windows-x64-web-setup.exe"
    $what = 'the AURORA {0} installer, which downloads only the apps you choose (windows-x64)' -f $version
    if ($env:AURORA_FULL_SETUP -or -not (& $listedAs $name)) {
        $name = "aurora-$version-windows-x64-setup.exe"
        $what = "AURORA $version (windows-x64)"
    }
    $setup = Join-Path $dir $name

    try {
        Write-Host "Downloading $what"
        try {
            Get-WithProgress "$base/$name" $setup
        } catch {
            # Uma resposta de erro do servidor (404, 403) não muda numa segunda
            # tentativa; uma conexão que caiu, sim. Ela vai pelo caminho de
            # antes, sem progresso.
            $failure = $_.Exception
            while ($failure -and -not ($failure -is [Net.WebException])) { $failure = $failure.InnerException }
            if ($failure -and $failure.Response) { throw }
            Write-Host ''
            Write-Host "The download failed ($($_.Exception.Message)); trying again without the progress line"
            Invoke-WebRequest -UseBasicParsing -Uri "$base/$name" -OutFile $setup
        }

        Write-Host 'Checking the SHA-256'
        $line = & $listedAs $name
        if (-not $line) { throw "The release SHA256SUMS does not list $name" }
        $expected = ($line -split '\s+')[0].ToLower()
        $actual = (Get-FileHash -Algorithm SHA256 -Path $setup).Hash.ToLower()
        if ($actual -ne $expected) { throw "The SHA-256 of $name does not match the release SHA256SUMS" }
    } catch {
        # Um instalador pela metade ou com o SHA-256 errado não fica no TEMP.
        Remove-Item -Recurse -Force $dir -ErrorAction SilentlyContinue
        throw
    }

    Write-Host 'Opening the installer'
    if ($env:AURORA_SETUP_ARGS) {
        $p = Start-Process -FilePath $setup -ArgumentList $env:AURORA_SETUP_ARGS -Wait -PassThru
    } else {
        $p = Start-Process -FilePath $setup -Wait -PassThru
    }
    Remove-Item -Recurse -Force $dir -ErrorAction SilentlyContinue
    if ($p.ExitCode -ne 0) { throw "The installer exited with code $($p.ExitCode)" }
    Write-Host "Done. Open a new terminal and run: aurora tools --verify"
}
