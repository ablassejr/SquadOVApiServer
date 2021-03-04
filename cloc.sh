#!/bin/bash

cloc . --exclude-dir=".vscode,deps,docs,static,target" --exclude-ext=json --exclude-list-file=".clocignore"