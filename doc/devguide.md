// issues vscode-css-languageservice

src/parser/cssParser.ts 
    line 260
        this.create(...) should be this.createNode(...) ?
    line 894
        comment should be // @property
    line 999
        simplify if statement
    line 1316
        simplify if statement

Initial build
-------------

ln -s "$(pwd)/target/debug/csslancer" "location-in-PATH/csslancer" 

New-Item -ItemType SymbolicLink -Path "location-in-PATH\csslancer.exe" -Target "$(Get-Location)\target\debug\csslancer.exe"