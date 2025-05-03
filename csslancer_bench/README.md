

###
```bash
sudo zypper install gcc-c++
```
No, do:
```bash
sudo zypper install -t pattern devel_C_C++
```

### Install Python3

python3

### Install gperf

```bash
sudo zypper install gperf
```

Windows
```shell
winget install gnuwin32.gperf
```

### Install json5

For make_runtime_features.py

OpenSUSE
```bash
sudo zypper install python313-json5
```

### Install jinja2


For make_style_shorthands.py
```bash
sudo zypper install python313-jinja2
```

Windows
```shell
pip install jinja2
```

### Github release token OR download unicode-org/icu

env var GHPAT_RO



### IN blink/Source/css
| match                      | #results | #files
| -------------------------- | -------- | ------
| #include "(?!core/css)     | 1317     | 320
| #include "wtf              | 361      | 198
| #include "platform         | 201      | 121
| #include "config.h"        | 168      | 168
| #include "core/dom         | 141      | 67
| #include "core/style       | 87       | 46
| #include "core/frame       | 57       | 36
| #include "core/html        | 49       | 24
| #include "core/coreExport  | 48       | 48
| #include "core/fetch       | 40       | 19
| #include "core/animation   | 35       | 7
| #include "core/layout      | 33       | 23
| #include "bindings/core/v8 | 31       | 23
| #include "core/svg         | 17       | 9
| #include "core/Media       | 12       | 10
| #include "core/inspector   | 10       | 10
| #include "core/StyleProp   | 10       | 10
| #include "public/platform  | 7        | 7
| #include "core/testing     | 5        | 5
| #include "core/events      | 5        | 4
| #include "core/page        | 4        | 2
| #include "core/loader      | 3        | 3
| #include "core/Math        | 1        | 1
| #include "core/editing     | 1        | 1
| #include "core/XMLNames.h  | 1        | 1

#include "(?!(config|platform|wtf|core\/(css|html|dom|style|layout|platform|fetch|svg|testing|frame|coreExport|Math|Media|loader|inspector|events|page|animation)|bindings\/core\/v8|public\/platform))

### IN blink/Source/css/parser
| match                      | #results | #files
| -------------------------- | -------- | ------
| #include "(?!core/css)     | 107      | 46
| #include "wtf              | 32       | 23
| #include "config.h         | 26       | 26
| #include "platform         | 13       | 11
| #include "core/CoreExport.h| 10       | 10
| #include "core/frame       | 6        | 5
| #include "core/html        | 5        | 5
| #include "core/Media       | 4        | 4
| #include "core/Style       | 4        | 4
| #include "core/dom         | 3        | 2
| #include "core/layout      | 2        | 2
| #include "core/fetch       | 1        | 1
| #include "core/svg         | 1        | 1

#include "(?!core\/(css|frame|Media|Style|dom|svg|html|layout|fetch|coreExport)|wtf|platform|config)