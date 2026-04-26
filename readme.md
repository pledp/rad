# eradic
Eradic is a lightweight DICOM (Digital Imaging and Communications in Medicine) implementation written in Rust. 

## Features

### Upper Layer service provider (UL SCP) development

Eradic enables building flexible DICOM Upper Layer service providers for different underlying architectures. 

The Eradic DICOM state machine only takes events as input to handle state transistions and produce commands as outputs.

```rust
let mut conn = UpperLayerConnection::new_server(socket_addr.ip(), tcp.local_addr()?.ip());

// 
let pdu = deserialize_associate_pdu(&mut reader)?;

let command = conn.handle_event(Event::AssociateRequestPdu(pdu))

match command {
    Some(Command::AssociateAcceptPdu(response)) => ...
```

It is the callers responsibility to invoke events and handle produced commands.

## Resources
- [DICOM standard](https://www.dicomstandard.org/current)
- [DICOM Standard Browser](https://dicom.innolitics.com/ciods)
