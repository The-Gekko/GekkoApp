
## Especificaciones del Binario

El ejecutable `gekkoapp` compilado (ofrecido como release artefacto) es un binario **dinámicamente enlazado** de tipo ELF x86-64 PIE (Position Independent Executable).

*   **Dependencias dinámicas:** Requiere `libc.so.6`, `libgcc_s.so.1` y `/lib64/ld-linux-x86-64.so.2`. La GLIBC mínima real no se fija a mano: `scripts/build-release-bundle.sh` la deduce de los símbolos versionados de los binarios (`objdump -T` → mayor `GLIBC_x.y`) y la escribe en `platform.libc.minimum` del manifiesto, que es lo que comprueban `installer.rs` e `install-release.sh` antes de instalar. Con la toolchain actual los binarios exigen GLIBC 2.39. El release v1.1.0 publicado declara 2.34 por un fallback (sin `objdump`) que ya no existe, aunque sus binarios exigen 2.39; el 1.2.0 lo corrige y el script ahora aborta si no puede medirla.
*   **Seguridad y mitigaciones:** Cuenta por defecto con las protecciones NX (Non-Executable stack), GNU RELRO y `BIND_NOW` habilitadas por el compilador Rust.
*   **Optimización:** El binario se distribuye `stripped` para reducir el tamaño final de ejecución.

## Tema adaptativo (Matugen)

Implementado en `src/core/theme.rs` y consumido por la GUI (comando `theme_state` + evento `theme://changed`). **No instala nada**: integra la paleta Material You que el setup de matugen/HyDE del sistema ya genera en `~/.cache/matugen/colors-gtk.css`.

*   `detect_palette` — lee el CSS, parsea los `@define-color`, expone `available`, `source`, `dark` (luminancia del fondo) y el mapa de colores.
*   `watch_palette` — hilo que sondea el archivo y avisa cuando cambia (el wallpaper se regeneró con matugen) para re-tematizar la GUI en vivo.
*   La webview mapea la paleta a las variables CSS (`--bg`, `--panel`, `--border`, `--text`, `--muted`, `--accent`, `--red`) al abrir y ante cada evento; si no hay paleta, vuelve al tema oscuro por defecto.
*   Al compartir el mismo CSS que importan GTK3 y GTK4, el Control Center sigue los mismos colores que las demás aplicaciones del escritorio.
