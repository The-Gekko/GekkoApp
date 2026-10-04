# Kitsune en GekkoApp

Kitsune es un módulo opcional del entorno Kito. Se muestra como **Kitsune**,
usa el identificador y ejecutable `kitsune` y obtiene los releases estables de
`KitotsuMolina/kitsunev2.0`. No usa el repositorio antiguo `KitsuneV2`.

Está disponible en la selección gráfica y en la opción 4 del menú de terminal.
Una instalación nueva no lo selecciona automáticamente. Si ya está instalado,
la interfaz lo marca siguiendo el comportamiento de los demás módulos para
actualizarlo. KiUI y Kitsune Compositor siguen siendo la base del plan.

El flujo resuelve el release, valida identidad, versión, arquitectura, glibc,
dependencias entre módulos y capacidades obligatorias. Muestra el plan antes
de instalar, descarga y verifica los artefactos, instala los paquetes faltantes
y activa el ejecutable mediante el motor existente. El manifiesto de Kitsune
requiere Kitsune Compositor `>=0.1.4, <0.2.0`.

## Dependencias

| Capacidad | Arch | Solus |
|---|---|---|
| `runtime.gtk4` | `gtk4` | `libgtk-4` |
| `runtime.gtk4-layer-shell` | `gtk4-layer-shell` | `gtk4-layer-shell` |
| `audio.pulseaudio-tools` | `libpulse` | `pulseaudio-utils` |

El mapeo existente de `gpu.wgpu` añade las bibliotecas gráficas correspondientes.
`media.mpris` es opcional: no disponer de un reproductor no impide instalar.
Las herramientas `pactl` y `parec` usan el servidor de audio del usuario; el
instalador no sustituye PulseAudio/PipeWire ni instala Niri o DMS.

Referencias de paquetes:
[GTK4 layer shell en Arch](https://archlinux.org/packages/extra/x86_64/gtk4-layer-shell/),
[libpulse en Arch](https://archlinux.org/packages/extra/x86_64/libpulse/files/),
[GTK4 en Solus](https://github.com/getsolus/packages/blob/main/packages/l/libgtk-4/package.yml),
[herramientas PulseAudio en Solus](https://github.com/getsolus/packages/blob/main/packages/p/pulseaudio/package.yml).

El lanzador habitual es `~/.local/bin/kitsune` (respetando las rutas del
instalador). KiUI ya recibe `KIUI_KITSUNE_BIN` y `KITSUNE_COMPOSITOR_BIN` en su
lanzador. Kitsune participa en el catálogo de versiones/actualizaciones y en
la desinstalación del entorno Kito; los datos de configuración del usuario
permanecen fuera de los archivos de instalación administrados.

## Verificación

```bash
cd 'Gekko APP/gekkoapp-rs'
cargo test --locked --all-targets --all-features
cargo clippy --locked --all-targets --all-features -- -D warnings
node --test tests/kito-ui.test.cjs
```

Las pruebas cubren selección opcional, compatibilidad de selecciones antiguas,
catálogo gráfico, envío de selección y mapeo de dependencias.

Prueba con el manifiesto y tar descargados en el mismo directorio (extrae y
activa únicamente en un directorio temporal, sin instalar paquetes):

```bash
GEKKOAPP_KITSUNE_MANIFEST=/ruta/kitsune-0.1.0-x86_64-unknown-linux-gnu.manifest.json \
  cargo test --all-features consumes_kitsune_release_and_activates_its_entrypoint -- --ignored
```

Para verificar por red el plan real con KiUI y Compositor, sin instalar:

```bash
cargo test --all-features resolves_published_kitsune_with_its_base_dependencies -- --ignored
```

Validado el 3 de octubre de 2026: 34 pruebas Rust, 5 pruebas de interfaz,
formato y Clippy sin errores. También pasaron las dos pruebas específicas:
resolución del plan por GitHub y extracción/activación temporal del paquete
público [Kitsune v0.1.0](https://github.com/KitotsuMolina/kitsunev2.0/releases/tag/v0.1.0).
No se instaló el módulo ni se cambiaron paquetes en la sesión del usuario.
