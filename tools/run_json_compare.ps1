param(
    [string[]]$Samples = @("mpxj/junit/data/legacy/sample.mpx", "mpxj/junit/data/legacy/sample.mpp")
)

$ErrorActionPreference = "Stop"
$repo = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$output = Join-Path $repo "_tmp_json_compare"
$classes = Join-Path $output "classes"
New-Item -ItemType Directory -Force -Path $classes | Out-Null

$upstream = Join-Path $repo "mpxj"
$mavenCommand = Get-Command mvn.cmd -ErrorAction SilentlyContinue
$maven = if ($mavenCommand) { $mavenCommand.Source } else { Join-Path $env:USERPROFILE "scoop/apps/maven/current/bin/mvn.cmd" }
if (-not (Test-Path $maven)) { throw "未找到 Maven: $maven" }
& $maven -q -DskipTests '-Dmaven.javadoc.skip=true' -f (Join-Path $upstream "pom.xml") package
if ($LASTEXITCODE -ne 0) { throw "Java 子模块源码构建失败" }

$upstreamJar = Join-Path $upstream "target/mpxj-16.7.0.jar"
$lib = Join-Path $upstream "lib"
if (-not (Test-Path $upstreamJar) -or -not (Test-Path $lib)) {
    throw "Maven 构建后缺少 MPXJ jar 或 lib/ 依赖"
}
$dependencies = @(Get-ChildItem $lib -Filter "*.jar" -File |
    Sort-Object Name | Select-Object -ExpandProperty FullName)
if ($dependencies.Count -eq 0) { throw "Java 子模块 lib/ 缺少运行时依赖" }
$classpath = (@($classes, $upstreamJar) + $dependencies) -join [IO.Path]::PathSeparator

& javac -encoding UTF-8 -cp $classpath -d $classes (Join-Path $PSScriptRoot "JsonSnapshot.java")
if ($LASTEXITCODE -ne 0) { throw "Java 快照编译失败" }
& cargo build --example json_snapshot --manifest-path (Join-Path $repo "Cargo.toml")
if ($LASTEXITCODE -ne 0) { throw "Rust 快照编译失败" }

$rustExe = Join-Path $repo "target/debug/examples/json_snapshot.exe"
$failed = 0
foreach ($sample in $Samples) {
    $samplePath = if ([IO.Path]::IsPathRooted($sample)) { $sample } else { Join-Path $repo $sample }
    $path = (Resolve-Path $samplePath).Path
    $stem = $sample -replace '[\\/:]', '_'
    $javaJson = Join-Path $output "$stem.java.json"
    $rustJson = Join-Path $output "$stem.rust.json"
    Write-Host "对拍: $sample"
    & java -cp $classpath JsonSnapshot $path | Out-File -Encoding utf8 $javaJson
    if ($LASTEXITCODE -ne 0) { throw "Java 读取失败: $sample" }
    & $rustExe $path | Out-File -Encoding utf8 $rustJson
    if ($LASTEXITCODE -ne 0) { throw "Rust 读取失败: $sample" }
    & python -X utf8 -B (Join-Path $PSScriptRoot "compare_json.py") $javaJson $rustJson
    if ($LASTEXITCODE -eq 1) { $failed++ }
    elseif ($LASTEXITCODE -ne 0) { throw "JSON 对拍工具失败: $sample" }
}
Write-Host "原始 JSON: $output"
if ($failed -ne 0) { exit 1 }
