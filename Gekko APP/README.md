
## Especificaciones del Binario

El ejecutable `gekkoapp` compilado (ofrecido como release artefacto) es un binario **dinámicamente enlazado** de tipo ELF x86-64 PIE (Position Independent Executable).

*   **Dependencias dinámicas:** Requiere `libc.so.6`, `libgcc_s.so.1` y `/lib64/ld-linux-x86-64.so.2` (con compatibilidad de símbolos hasta GLIBC 2.39).
*   **Seguridad y mitigaciones:** Cuenta por defecto con las protecciones NX (Non-Executable stack), GNU RELRO y `BIND_NOW` habilitadas por el compilador Rust.
*   **Optimización:** El binario se distribuye `stripped` para reducir el tamaño final de ejecución.
