$vars = Get-Content $PSScriptRoot/dev_vars.json | ConvertFrom-Json

$vars.PSObject.Properties | ForEach-Object -Process {
    $KEY=$_.Name
    Set-Item -LiteralPath Env:$KEY -Value $_.Value
}