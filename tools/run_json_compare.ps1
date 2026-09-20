param(
    [string[]]$Samples,
    [switch]$AllFields,
    [switch]$Quiet,
    [ValidateRange(1, 32)]
    [int]$Parallelism = 1
)

$ErrorActionPreference = "Stop"
$repo = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$pythonCommand = Get-Command python.exe -ErrorAction SilentlyContinue
if (-not $pythonCommand) { $pythonCommand = Get-Command python -ErrorAction SilentlyContinue }
if (-not $pythonCommand) { throw "未找到 Python" }

$pythonArgs = @(
    "-X", "utf8", "-B",
    (Join-Path $PSScriptRoot "run_json_compare.py"),
    "--parallelism", $Parallelism
)
if ($AllFields) { $pythonArgs += "--all-fields" }
if ($Quiet) { $pythonArgs += "--quiet" }

if ($PSBoundParameters.ContainsKey("Samples")) {
    if ($Samples.Count -eq 0) { throw "没有待对拍的样本" }
    $output = Join-Path $repo "_tmp_json_compare"
    New-Item -ItemType Directory -Force -Path $output | Out-Null
    $sampleList = Join-Path $output "samples-from-powershell.txt"
    $Samples | Set-Content -Encoding utf8 -Path $sampleList
    $pythonArgs += @("--sample-list", $sampleList)
}

& $pythonCommand.Source @pythonArgs
exit $LASTEXITCODE
