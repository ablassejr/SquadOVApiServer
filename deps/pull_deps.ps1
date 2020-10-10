$url = "https://repo1.maven.org/maven2/org/flywaydb/flyway-commandline/7.0.0/flyway-commandline-7.0.0-windows-x64.zip"
$output = "flyway.zip"

Invoke-WebRequest -Uri $url -OutFile $output
Expand-Archive -Path $output -DestinationPath flyway_tmp
Move-Item -Path flyway_tmp/flyway-7.0.0 -Destination flyway
Remove-Item flyway_tmp -Recurse
Remove-Item $output