

// We fuzz 2 things
// ## 1. Reparsing vs. parsing
// We fuzz test that incremental reparsing generates the same syntax tree and
// errors as parsing. 
// # 2. Csslancer vs. Firefox
// We fuzz test that any css file which parses without errors in Firefox' 
// parser also parses without errors in our parser.

// # Integration testing
// Pull css files from repositories and top websites and parse them. Files with
// parsing errors are collected for review.


// pub fn collect_files() -> Vec<&str> {
    
// }

// pub fn generate_plausible_css() -> &str {
    
// }

use rowan::TextSize;

use super::parse_source_file_text;

pub fn init() {
    let seed = fastrand::u64(..);
    fastrand::seed(seed);
    println!("SEED: {seed}");

    let css = all_css();
    let (_node, errs) = parse_source_file_text(&css);

    assert!(errs.is_empty(), "expected no errs, found {}", errs.iter().fold(String::new(), 
        |acc, nex| format!("{acc}\n- {} : {:?} {:?} : `{}`", 
            nex.to_string(), 
            nex.range().start(), 
            nex.range().end(), 
            &css[
                (<TextSize as Into<usize>>::into(nex.range().start())-10).min(css.len()-1)..
                (<TextSize as Into<usize>>::into(nex.range().end())+10).min(css.len())
            ]
        )
    ));
    println!("CSS SRC PARSED WITHOUT PANIC/PARSE ERROR");
    for _ in 0..1000 {
        let mut css = all_css();
        mutate_ascii(&mut css);
        parse_source_file_text(&css);
    }
    println!("PARSED X MUTATIONS WITHOUT PANIC");
}

pub fn mutate_ascii(text: &mut String) {
    let chars_num = text.chars().count();

    let copy_range_char_start = fastrand::usize(0..chars_num);
    let copy_range_char_end = fastrand::usize(copy_range_char_start..chars_num);
    let del_range_char_start = fastrand::usize(0..chars_num);
    let del_range_char_end = fastrand::usize(del_range_char_start..text.len());

    let mut copy_range_idx_start = 0;
    let mut copy_range_idx_end = 0;
    let mut del_range_idx_start = 0;
    let mut del_range_idx_end = 0;
    for (i, ch) in text.char_indices() {
      let bytes_size = ch.len_utf8();
      if i <= copy_range_char_start {
        copy_range_idx_start += bytes_size;
      }
      if i <= copy_range_char_end {
        copy_range_idx_end += bytes_size;
      } 
      if i <= del_range_char_start {
        del_range_idx_start += bytes_size;
      }
      if i <= del_range_char_end {
        del_range_idx_end += bytes_size;
      }
    }

    let replace_text = text[copy_range_idx_start..copy_range_idx_end].to_owned();

    text.replace_range(del_range_idx_start..del_range_idx_end, &replace_text);
}

pub fn all_css() -> String {
    AT_RULES_CSS.to_owned() +
        ATTRIBUTE_SELECTORS_CSS + 
        CLASS_SELECTORS_CSS + 
        ID_SELECTORS_CSS + 
        NESTED_CSS_SELECTORS +
        TYPE_SELECTORS_CSS +
        UNIVERSAL_SELECTORS_CSS
}


const AT_RULES_CSS: &str = r##"@charset "utf-8";
@color-profile --swop5c {
    src: url("https://example.org/SWOP2006_Coated5v2.icc");
}

@container (width > 400px) {
    h2 {
      font-size: 1.5em;
    }
}
/* with an optional <container-name> */
@container tall (height > 30rem) {
    h2 {
        line-height: 1.6;
    }
}
@container (width > 400px) and (height > 400px) {
    /* <stylesheet> */
}

@container (width > 400px) or (height > 400px) {
    /* <stylesheet> */
}

@container not (width < 400px) {
    /* <stylesheet> */
}
/* Apply styles if the container is narrower than 650px */
@container (width < 650px) {
  .card {
    width: 50%;
    background-color: gray;
    font-size: 1em;
  }
}


@counter-style thumbs {
    system: cyclic;
    symbols: "\1F44D";
    suffix: " ";
}
@counter-style circled-alpha {
    system: fixed;
    symbols: Ⓐ Ⓑ Ⓒ Ⓓ Ⓔ Ⓕ Ⓖ Ⓗ Ⓘ Ⓙ Ⓚ Ⓛ Ⓜ Ⓝ Ⓞ Ⓟ Ⓠ Ⓡ Ⓢ Ⓣ Ⓤ Ⓥ Ⓦ Ⓧ Ⓨ Ⓩ;
    suffix: " ";
}

@document url("https://www.example.com/")
{
  h1 {
    color: green;
  }
}
@document url("http://www.w3.org/"),
          url-prefix("http://www.w3.org/Style/"),
          domain("mozilla.org"),
          media-document("video"),
          regexp("https:.*") {
  /* CSS rules here apply to:
     - The page "http://www.w3.org/"
     - Any page whose URL begins with "http://www.w3.org/Style/"
     - Any page whose URL's host is "mozilla.org"
       or ends with ".mozilla.org"
     - Any standalone video
     - Any page whose URL starts with "https:" */

  /* Make the above-mentioned pages really ugly */
  body {
    color: purple;
    background: yellow;
  }
}


@font-face {
    font-family: "Trickster";
    src:
      local("Trickster"),
      url("trickster-COLRv1.otf") format("opentype") tech(color-COLRv1),
      url("trickster-outline.otf") format("opentype"),
      url("trickster-outline.woff") format("woff");
}
@font-face {
    font-family: "MyHelvetica";
    src: local("Helvetica Neue Bold"), local("HelveticaNeue-Bold"),
      url("MgOpenModernaBold.ttf");
    font-weight: bold;
}
   

/* At-rule for "nice-style" in Font One */
@font-feature-values Font One {
  @styleset {
    nice-style: 12;
  }
}

/* At-rule for "nice-style" in Font Two */
@font-feature-values Font Two {
  @styleset {
    nice-style: 4;
  }
}



/* Apply the at-rules with a single declaration */
.nice-look {
  font-variant-alternates: styleset(nice-style);
}


@font-palette-values --identifier {
    font-family: Bixa;
}
@import url(https://fonts.googleapis.com/css2?family=Bungee+Spice);
p {
  font-family: "Bungee Spice";
  font-size: 2rem;
}
@font-palette-values --Alternate {
  font-family: "Bungee Spice";
  override-colors:
    0 #00ffbb,
    1 #007744;
}

/*
@import url;
@import url layer;
@import url layer(layer-name);
@import url layer(layer-name) supports(supports-condition);
@import url layer(layer-name) supports(supports-condition) list-of-media-queries;
@import url layer(layer-name) list-of-media-queries;
@import url supports(supports-condition);
@import url supports(supports-condition) list-of-media-queries;
@import url list-of-media-queries;
*/

@import url("my-imported-styles.css");
* {
  margin: 0;
  padding: 0;
}
@import "custom.css";
@import url("chrome://communicator/skin/");
@import url("fineprint.css") print;
@import url("bluish.css") print, screen;
@import "common.css" screen;
@import url("landscape.css") screen and (orientation: landscape);
@import url("gridy.css") supports(display: grid) screen and (max-width: 400px);
@import url("flexy.css") supports((not (display: grid)) and (display: flex)) screen
  and (max-width: 400px);
@import url("whatever.css") supports((selector(h2 > p)) and
  (font-tech(color-COLRv1)));
@import "theme.css" layer(utilities);
@import url(headings.css) layer(default);
@import url(links.css) layer(default);

@layer default {
  audio[controls] {
    display: block;
  }
}
@import "theme.css" layer();
@import "style.css" layer;




@keyframes slidein {
    from {
      transform: translateX(0%);
    }
  
    to {
      transform: translateX(100%);
    }
}
@keyframes identifier {
    0% {
      top: 0;
      left: 0;
    }
    30% {
      top: 50px;
    }
    68%,
    72% {
      left: 50px;
    }
    100% {
      top: 100px;
      left: 100%;
    }
}
@keyframes identifier {
    0% {
      top: 0;
    }
    50% {
      top: 30px;
      left: 20px;
    }
    50% {
      top: 10px;
    }
    100% {
      top: 0;
    }
}
@keyframes important1 {
    from {
      margin-top: 50px;
    }
    50% {
      margin-top: 150px !important; /* ignored */
    }
    to {
      margin-top: 100px;
    }
}
  
@keyframes important2 {
    from {
      margin-top: 50px;
      margin-bottom: 100px;
    }
    to {
      margin-top: 150px !important; /* ignored */
      margin-bottom: 50px;
    }
}
  
    



@layer utilities {
    .padding-sm {
      padding: 0.5rem;
    }
  
    .padding-lg {
      padding: 0.8rem;
    }
}
@layer utilities;
@layer theme, layout, utilities;
@layer {
    p {
      margin-block: 1rem;
    }
}
@import "theme.css" layer(utilities);
@layer framework {
    @layer layout {
    }
}
@layer framework.layout {
    p {
      margin-block: 1rem;
    }
}
p {
    color: rebeccapurple;
}
@layer type {
    .box p {
      font-weight: bold;
      font-size: 1.3em;
      color: green;
    }
}






@media (hover: hover) {
    abbr:hover {
      color: limegreen;
      transition-duration: 1s;
    }
}
  
@media not all and (hover: hover) {
    abbr::after {
      content: ' (' attr(title) ')';
    }
}
/* At the top level of your code */
@media screen and (min-width: 900px) {
  article {
    padding: 1rem 3rem;
  }
}

/* Nested within another conditional at-rule */
@supports (display: flex) {
  @media screen and (min-width: 900px) {
    article {
      display: flex;
    }
  }
}
@media print {
    body {
      font-size: 10pt;
    }
}
  
@media screen {
    body {
      font-size: 13px;
    }
}
  
@media screen, print {
    body {
      line-height: 1.2;
    }
}
  
@media only screen and (min-width: 320px) and (max-width: 480px) and (resolution: 150dpi) {
    body {
      line-height: 1.4;
    }
}
@media (height > 600px) {
    body {
      line-height: 1.4;
    }
}  
@media (400px <= width <= 700px) {
    body {
      line-height: 1.4;
    }
}
  
  




@namespace url(http://www.w3.org/1999/xhtml);
@namespace svg url(http://www.w3.org/2000/svg);

/* This matches all XHTML <a> elements, as XHTML is the default unprefixed namespace */
a {
}

/* This matches all SVG <a> elements */
svg|a {
}

/* This matches both XHTML and SVG <a> elements */
*|a {
}






/* Targets all the pages */
@page {
  size: 8.5in 9in;
  margin-top: 4in;
}

/* Targets all even-numbered pages */
@page :left {
  margin-top: 4in;
}

/* Targets all odd-numbered pages */
@page :right {
  size: 11in;
  margin-top: 4in;
}

/* Targets all selectors with `page: wide;` set */
@page wide {
  size: a4 landscape;
}

@page {
  /* margin box at top right showing page number */
  @top-right {
    content: "Page " counter(pageNumber);
  }
}
@page {
    @top-left {
      /* page-margin-properties */
    }
}
@page {
    size: landscape;
    margin: 20%;
}
  
section {
    page-break-after: always;
    break-after: page;
}
  
@media print {
    button {
      display: none;
    }
}
    
  






@property --property-name {
    syntax: "<color>";
    inherits: false;
    initial-value: #c0ffee;
}
@property --item-size {
    syntax: "<percentage>";
    inherits: true;
    initial-value: 40%;
}
.container {
    display: flex;
    height: 200px;
    border: 1px dashed black;
  
    /* set custom property values on parent */
    --item-size: 20%;
    --item-color: orange;
}

/* use custom properties to set item size and background color */
.item {
    width: var(--item-size);
    height: var(--item-size);
    background-color: var(--item-color);
}

/* set custom property values on element itself */
.two {
    --item-size: initial;
    --item-color: inherit;
}

.three {
    /* invalid values */
    --item-size: 1000px;
    --item-color: xyz;
}
  








@scope (scope root) to (scope limit) {
    rulesets
}
@scope (.article-body) to (figure) {
    img {
        border: 5px solid black;
        background-color: goldenrod;
    }
}


@scope (.article-body) {
    img {
      border: 5px solid black;
      background-color: goldenrod;
    }
}

  
@scope (.feature) {
    :scope {
        background: rebeccapurple;
        color: antiquewhite;
        font-family: sans-serif;
    }
}

/* figure is only a limit when it is a direct child of the :scope */
@scope (.article-body) to (:scope > figure) { }
/* figure is only a limit when the :scope is inside .feature */
@scope (.article-body) to (.feature :scope figure) { }

@scope (.article-hero, .article-body) to (figure) {
    img {
      border: 5px solid black;
      background-color: goldenrod;
    }
}
@scope (.article-body) {
    /* img has a specificity of 0-0-1, as expected */
    img { }
}
@scope (figure, #primary) {
    & img { }
}
  
@scope (.feature) {
    /* Selects a .feature inside the matched root .feature */
    & & { ... }
  
    /* Doesn't work */
    :scope :scope { ... }
}
@scope (.light-theme) {
    :scope {
      background: #ccc;
    }
    p {
      color: black;
    }
  }
  
  @scope (.dark-theme) {
    :scope {
      background: #333;
    }
    p {
      color: white;
    }
}
@scope (.light-scheme) {
    :scope {
      background-color: plum;
    }
  
    a {
      color: darkmagenta;
    }
}
  
@scope (.dark-scheme) {
    :scope {
      background-color: darkmagenta;
      color: antiquewhite;
    }
  
    a {
      color: plum;
    }
}
/* Scoped CSS */

@scope (.feature) {
  :scope {
    background: rebeccapurple;
    color: antiquewhite;
    font-family: sans-serif;
  }

  figure {
    background-color: white;
    border: 2px solid black;
    color: black;
    padding: 10px;
  }
}

/* Donut scope */

@scope (.feature) to (figure) {
  img {
    border: 5px solid black;
    background-color: goldenrod;
  }
}











[popover]:popover-open {
    opacity: 1;
    transform: scaleX(1);
}
@starting-style {
    [popover]:popover-open {
      opacity: 0;
      transform: scaleX(0);
    }
}
[popover]:popover-open {
    opacity: 1;
    transform: scaleX(1);
  
    @starting-style {
      opacity: 0;
      transform: scaleX(0);
    }
}
#target {
    transition: background-color 1.5s;
    background-color: green;
  }
  
  @starting-style {
    #target {
      background-color: transparent;
    }
}

#target {
    transition-property: opacity, display;
    transition-duration: 0.5s;
    display: block;
    opacity: 1;
    @starting-style {
      opacity: 0;
    }
  }
  
  #target.hidden {
    display: none;
    opacity: 0;
}

div {
    background-color: yellow;
    transition: background-color 3s;
  }
  
  div.showing {
    background-color: skyblue;
  }
  
  @starting-style {
    div.showing {
      background-color: red;
    }
}
html {
    font-family: Arial, Helvetica, sans-serif;
  }
  
  [popover]:popover-open {
    opacity: 1;
    transform: scaleX(1);
  }
  
  [popover] {
    font-size: 1.2rem;
    padding: 10px;
  
    /* Final state of the exit animation */
    opacity: 0;
    transform: scaleX(0);
  
    transition:
      opacity 0.7s,
      transform 0.7s,
      overlay 0.7s allow-discrete,
      display 0.7s allow-discrete;
    /* Equivalent to
    transition: all 0.7s allow-discrete; */
  }
  
  /* Include after the [popover]:popover-open rule */
  @starting-style {
    [popover]:popover-open {
      opacity: 0;
      transform: scaleX(0);
    }
  }
  
  /* Transition for the popover's backdrop */
  [popover]::backdrop {
    background-color: rgb(0 0 0 / 0%);
    transition:
      display 0.7s allow-discrete,
      overlay 0.7s allow-discrete,
      background-color 0.7s;
    /* Equivalent to
    transition: all 0.7s allow-discrete; */
  }
  
  [popover]:popover-open::backdrop {
    background-color: rgb(0 0 0 / 25%);
  }
  
  /* Nesting (&) is not supported for pseudo-elements
  so specify a standalone starting-style block. */
  @starting-style {
    [popover]:popover-open::backdrop {
      background-color: rgb(0 0 0 / 0%);
    }
}

div {
    flex: 1;
    border: 1px solid gray;
    position: relative;
    background: linear-gradient(
      to right,
      rgb(255 255 255 / 0%),
      rgb(255 255 255 / 50%)
    );
    opacity: 1;
    scale: 1 1;
  
    transition:
      opacity 0.7s,
      scale 0.7s,
      display 0.7s allow-discrete
      all 0.7s allow-discrete;
    /* Equivalent to
    transition: all 0.7s allow-discrete; */
  }
  
  /* Include after the `div` rule */
  @starting-style {
    div {
      opacity: 0;
      scale: 1 0;
    }
  }
  
  .fade-out {
    opacity: 0;
    display: none;
    scale: 1 0;
  }
  
  div > button {
    font-size: 1.6rem;
    background: none;
    border: 0;
    text-shadow: 2px 1px 1px white;
    border-radius: 15px;
    position: absolute;
    top: 1px;
    right: 1px;
    cursor: pointer;
}











@supports (display: flex) {
    .flex-container > * {
      text-shadow: 0 0 2px blue;
      float: none;
    }
  
    .flex-container {
      display: flex;
    }
}


@supports (transform-origin: 5% 5%) {
}
@supports selector(h2 > p) {
}
@supports font-tech(color-COLRv1) {
}
@supports font-format(opentype) {
}
@supports not (transform-origin: 10em 10em 10em) {
}
@supports not (not (transform-origin: 2px)) {
}
@supports (display: grid) and (not (display: inline-grid)) {
}
@supports (display: table-cell) and (display: list-item) {
}
@supports (display: table-cell) and (display: list-item) and (display: contents) {
}
@supports (display: table-cell) and
  ((display: list-item) and (display: contents)) {
}
@supports (transform-style: preserve) or (-moz-transform-style: preserve) {
}

@supports (transform-style: preserve) or (-moz-transform-style: preserve) or (-webkit-transform-style: preserve) {}

@supports (transform-style: preserve-3d) or ((-moz-transform-style: preserve-3d) or (-webkit-transform-style: preserve-3d)) {}

@supports (animation-name: test) {
    /* CSS applied when animations are supported without a prefix */
    @keyframes my-animation {
      /* Other at-rules can be nested inside */
    }
}

@supports (text-stroke: 10px) or (-webkit-text-stroke: 10px) {
    /* CSS applied when text-stroke, prefixed or not, is supported */
}
  
@supports not ((text-align-last: justify) or (-moz-text-align-last: justify)) {
    /* CSS to provide fallback alternative for text-align-last: justify */
}
/* This rule won't be applied in browsers that don't support :has() */
ul:has(> li li) {
  /* CSS is applied when the :has(...) pseudo-class is supported */
}

@supports not selector(:has(a, b)) {
  /* Fallback for when :has() is unsupported */
  ul > li,
  ol > li {
    /* The above expanded for browsers that don't support :has(...) */
  }
}

/* Note: So far, there's no browser that supports the `of` argument of :nth-child(...) */
@supports selector(:nth-child(1n of a, b)) {
  /* This rule needs to be inside the @supports block, otherwise
     it will be partially applied in browsers which don't support
     the `of` argument of :nth-child(...) */
  :is(:nth-child(1n of ul, ol) a, details > summary) {
    /* CSS applied when the :is(...) selector and
       the `of` argument of :nth-child(...) are both supported */
  }
}

@import url("https://fonts.googleapis.com/css2?family=Bungee+Spice");

@supports font-tech(color-COLRv1) {
  body {
    font-family: "Bungee Spice";
  }
}
@font-face {
    font-family: "Bungee Spice";
    src:
      url("https://fonts.googleapis.com/css2?family=Bungee+Spice") tech(color-COLRv1),
      url("Bungee-fallback.otf") format("opentype");
}


@supports font-format(woff2) {
  body {
    font-family: "Open Sans";
    src: url("open-sans.woff2") format("woff2");
  }
} 
"##;




const ATTRIBUTE_SELECTORS_CSS: &str = r##"
/* <a> elements with a title attribute */
a[title] {
  color: purple;
}

/* <a> elements with an href matching "https://example.org" */
a[href="https://example.org"]
{
  color: green;
}

/* <a> elements with an href containing "example" */
a[href*="example"] {
  font-size: 2em;
}

/* <a> elements with an href ending ".org", case-insensitive */
a[href$=".org" i] {
  font-style: italic;
}

/* <a> elements whose class attribute contains the word "logo" */
a[class~="logo"] {
  padding: 2px;
}



a {
    color: blue;
  }
  
  /* Internal links, beginning with "#" */
  a[href^="#"] {
    background-color: gold;
  }
  
  /* Links with "example" anywhere in the URL */
  a[href*="example"] {
    background-color: silver;
  }
  
  /* Links with "insensitive" anywhere in the URL,
     regardless of capitalization */
  a[href*="insensitive" i] {
    color: cyan;
  }
  
  /* Links with "cAsE" anywhere in the URL,
  with matching capitalization */
  a[href*="cAsE" s] {
    color: pink;
  }
  
  /* Links that end in ".org" */
  a[href$=".org"] {
    color: red;
  }
  
  /* Links that start with "https://" and end in ".org" */
  a[href^="https://"][href$=".org"]
  {
    color: green;
  }
  

  /* All divs with a `lang` attribute are bold. */
div[lang] {
  font-weight: bold;
}

/* All divs without a `lang` attribute are italicized. */
div:not([lang]) {
  font-style: italic;
}

/* All divs in US English are blue. */
div[lang~="en-us"] {
  color: blue;
}

/* All divs in Portuguese are green. */
div[lang="pt"] {
  color: green;
}

/* All divs in Chinese are red, whether
   simplified (zh-Hans-CN) or traditional (zh-Hant-TW). */
div[lang|="zh"] {
  color: red;
}

/* All divs with a Traditional Chinese
   `data-lang` are purple. */
/* Note: You could also use hyphenated attributes
   without double quotes */
div[data-lang="zh-Hant-TW"] {
  color: purple;
}


/* Case-sensitivity depends on document language */
ol[type="a"]:first-child {
  list-style-type: lower-alpha;
  background: red;
}

ol[type="i" s] {
  list-style-type: lower-alpha;
  background: lime;
}

ol[type="I" s] {
  list-style-type: upper-alpha;
  background: grey;
}

ol[type="a" i] {
  list-style-type: upper-alpha;
  background: green;
}
"##;



const CLASS_SELECTORS_CSS: &str = r##"
/* All elements with class="spacious" */
.spacious {
  margin: 2em;
}

/* All <li> elements with class="spacious" */
li.spacious {
  margin: 2em;
}

/* All <li> elements with a class list that includes both "spacious" and "elegant" */
/* For example, class="elegant retro spacious" */
li.spacious.elegant {
  margin: 2em;
}
.red {
    color: #f33;
}

.yellow-bg {
    background: #ffa;
}

.fancy {
    font-weight: bold;
    text-shadow: 4px 4px 3px #77f;
}  
"##;



const ID_SELECTORS_CSS: &str = r##"
/* The element with id="demo" */
#demo {
  border: red 2px solid;
}
#identified {
    background-color: skyblue;
}  
"##;

const NESTED_CSS_SELECTORS: &str = r##"
parentRule {
    /* parent rule style properties */
    & childRule {
      /* child rule style properties */
    }
}
.parent-rule {
    /* parent rule properties */
    .child-rule {
      /* child rule properties */
    }
}
.parent-rule {
    /* parent rule properties */
    :hover {
      /* child rule properties */
    }
  }
  
  /* the browser parses the above nested rules as shown below */
  .parent-rule {
    /* parent rule properties */
  }
  
  .parent-rule *:hover {
    /* child rule properties */
}
.parent-rule {
    /* parent rule properties */
    &:hover {
      /* child rule properties */
    }
  }
  
  /* the browser parses the above nested rules as shown below */
  .parent-rule {
    /* parent rule properties */
  }
  
  .parent-rule:hover {
    /* child rule properties */
}
.card {
    /* .card styles */
    .featured & {
      /* .featured .card styles */
    }
  }
  
  /* the browser parses above nested rules as */
  
  .card {
    /* .card styles */
  }
  
  .featured .card {
    /* .featured .card styles */
}
.card {
    /* .card styles */
    .featured & & & {
      /* .featured .card .card .card styles */
    }
  }
  
  /* the browser parses above nested rules as */
  
  .card {
    /* .card styles */
  }
  
  .featured .card .card .card {
    /* .featured .card .card .card styles */
}

.example {
    font-family: system-ui;
    font-size: 1.2rem;
  }
  
  .example > a {
    color: tomato;
  }
  
  .example > a:hover,
  .example > a:focus {
    color: ivory;
    background-color: tomato;
}

.example {
    font-family: system-ui;
    font-size: 1.2rem;
    & > a {
      color: tomato;
      &:hover,
      &:focus {
        color: ivory;
        background-color: tomato;
      }
    }
}
& {
    color: blue;
    font-weight: bold;
}
  
&:hover {
    background-color: wheat;
}
"##;


const TYPE_SELECTORS_CSS: &str = r##"
/* All <a> elements. */
a {
  color: red;
}
span {
    background-color: skyblue;
}
@namespace example url(http://www.example.com);
example|h1 {
  color: blue;
}
"##;

const UNIVERSAL_SELECTORS_CSS: &str = r##"
* [lang^="en"] {
    color: green;
}
  
*.warning {
    color: red;
}
  
*#maincontent {
    border: 1px solid blue;
}
  
.floating {
    float: left;
}
  
/* automatically clear the next sibling after a floating element */
.floating + * {
    clear: left;
}

@namespace example url(http://www.example.com);
example|* {
  color: blue;
}
"##;
