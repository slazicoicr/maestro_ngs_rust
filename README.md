# Maestro NGS Rust

Emulate a [Sciclone G3 NGS](https://www.perkinelmer.com/product/sciclone-g3-ngs-workstation-cls145321)
protocol. The input is the exported Maestro application `.eap` file. 

In development:
* `maestro_application`: library that stores a Maestro application in a Rust data 
structure
  
Planned:
* `maestro_emulator`: library that emulates a Maestro application
* `maestro_cli`: command line interface bin to interrogate a Maestro application
  