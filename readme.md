# eradic
Eradic is a DICOM (Digital Imaging and Communications in Medicine) implementation written in Rust. Eradic includes several crates to facilitate DICOM message passing over a network.

## Crates 

### [eradic_core](./crates/rad_common)

Includes core DICOM data structures. Acts as the core layer for a DICOM UL (upper layer) implementation.

### [eradic_adaptor](./crates/eradic_adaptor)

Part of DICOM service user layer. DICOM UL service provider does not know about 

### [eradic_ul](./crates/eradic_ul)

TODO: Move rad/src to crates/eradic_ul

DICOM UL service provider implementation server written with Tokio. Uses [eradic_core](./crates/rad_common) and [eradic_adaptor](./crates/eradic_adaptor).

### [eradic_client](./crates/rad_client)

Simple DICOM UL service provider used to facilitate testing.

## Features

### Resources
- [DICOM standard](https://www.dicomstandard.org/current)
- [DICOM Standard Browser](https://dicom.innolitics.com/ciods)
