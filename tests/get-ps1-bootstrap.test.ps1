<#
.SYNOPSIS
  Verifies that scripts/get.ps1 parses when README's irm bootstrap commands fetch it as a string.

.DESCRIPTION
  Invoke-RestMethod decodes the HTTP response without removing a UTF-8 BOM. A BOM before
  the opening block comment makes PowerShell parse the comment content as code. This test
  decodes get.ps1 exactly as irm does, then validates the supported invocation forms.

  The bootstrap script must also remain ASCII-only: when a BOM-free script is executed as
  a file by Windows PowerShell 5.1, the ANSI code page otherwise makes parsing inconsistent.

.EXAMPLE
  pwsh -NoProfile -File tests/get-ps1-bootstrap.test.ps1
#>
[CmdletBinding()]
param(
    [string]$Path = (Join-Path (Split-Path -Parent $PSScriptRoot) "scripts/get.ps1")
)

$ErrorActionPreference = "Stop"
$Failures = 0

function Pass([string]$Message) {
    Write-Host "ok - $Message"
}

function Fail([string]$Message) {
    [Console]::Error.WriteLine("FAIL - $Message")
    $script:Failures++
}

function Test-ParsesCleanly([string]$Source, [string]$Description) {
    $errors = $null
    [void][System.Management.Automation.Language.Parser]::ParseInput($Source, [ref]$null, [ref]$errors)
    if ($errors.Count -eq 0) {
        Pass "$Description parses without errors"
        return
    }

    $first = $errors[0]
    Fail ("{0} has {1} parse errors; first is on line {2}: {3}" -f $Description, $errors.Count, $first.Extent.StartLineNumber, $first.Message)
}

$bytes = [IO.File]::ReadAllBytes($Path)

if ($bytes.Length -ge 3 -and $bytes[0] -eq 0xEF -and $bytes[1] -eq 0xBB -and $bytes[2] -eq 0xBF) {
    Fail "get.ps1 starts with a UTF-8 BOM (EF BB BF), which causes a ParserError with irm | iex"
} else {
    Pass "get.ps1 has no UTF-8 BOM"
}

$nonAsciiIndex = [Array]::FindIndex($bytes, [Predicate[byte]] { param($b) $b -ge 0x80 })
if ($nonAsciiIndex -ge 0) {
    $line = 1 + @($bytes[0..$nonAsciiIndex] | Where-Object { $_ -eq 0x0A }).Count
    Fail "get.ps1 has a non-ASCII character on line $line"
} else {
    Pass "get.ps1 contains only ASCII characters"
}

$content = [Text.Encoding]::UTF8.GetString($bytes)
Test-ParsesCleanly $content "irm | iex"
Test-ParsesCleanly "& { $content } -InstallDir 'D:\Apps\Token Usage Insights' -Port 3010" 'Invoke-Expression "& { $script } ..."'

try {
    [void][scriptblock]::Create($content)
    Pass "[scriptblock]::Create((irm ...)) creates the script block"
} catch {
    Fail "[scriptblock]::Create((irm ...)) failed: $($_.Exception.Message)"
}

Write-Host ""
if ($Failures -ne 0) {
    [Console]::Error.WriteLine("$Failures checks failed")
    exit 1
}

Write-Host "get.ps1 bootstrap tests passed"
