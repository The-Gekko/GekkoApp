# Instalacion del entorno Kito

## Objetivo

GekkoApp instala artefactos publicados del ecosistema Kito. No implementa la
logica de wallpapers, monitores, servicios ni adaptadores de escritorio. Esa
responsabilidad corresponde a los CLI y a Kitsune Compositor.

## Flujo inicial

1. El usuario selecciona `Instalar entorno Kito`.
2. GekkoApp detecta `/etc/os-release`, arquitectura, tipo de sesion, escritorio,
   gestor de paquetes y disponibilidad de systemd.
3. El resultado se muestra antes de continuar y puede corregirse manualmente.
4. El usuario selecciona Kitowall y/o Kilivepaper. Kitsune se muestra como
   proximamente y no puede agregarse al plan.
5. KiUI y Kitsune Compositor se agregan obligatoriamente al plan.
6. GekkoApp consulta el release estable mas reciente de cada repositorio.
7. Descarga y valida todos los manifests, incluida la plataforma, glibc minima,
   dependencias modulares, payload e integraciones de escritorio.
8. Si falta un artefacto para el target detectado, se aborta antes de modificar
   archivos o instalar paquetes.
9. Presenta el plan y solicita confirmacion explicita.
10. Instala las dependencias de host faltantes mediante `pacman`.
11. Descarga los paquetes, verifica tamano y SHA-256, los extrae en staging y
    valida cada archivo contra el manifest.
12. Activa los entrypoints y la integracion de escritorio, y registra el estado.

## Matriz soportada en la primera version

| Dimension | Soporte |
| --- | --- |
| Distribucion | Arch Linux y derivadas |
| Arquitectura | x86_64 |
| Sesion | Wayland |
| Escritorio | Hyprland |
| Servicios | systemd de usuario |
| Target | x86_64-unknown-linux-gnu |

La deteccion de Ubuntu/Debian, Fedora, GNOME, KDE y Niri existe para generar un
diagnostico correcto, pero todavia no habilita su instalacion.

## Repositorios resueltos

| Componente | Repositorio | Tipo |
| --- | --- | --- |
| KiUI | KitotsuMolina/KiUI | obligatorio |
| Kitsune Compositor | KitotsuMolina/Kito-compositor | obligatorio |
| Kitowall | KitotsuMolina/KitowallV2 | seleccionable |
| Kilivepaper | KitotsuMolina/Kilivepaper | seleccionable |
| Kitsune | KitotsuMolina/KitsuneV2 | proximamente, deshabilitado |

GekkoApp no fija versiones en el codigo. Consulta GitHub Releases y exige un
archivo `*-<target>.manifest.json` en el release encontrado.

## Rutas de instalacion

| Contenido | Ruta predeterminada |
| --- | --- |
| Versiones inmutables | `~/.local/lib/kitotsu/<producto>/<version>/` |
| CLI activos | `~/.local/bin/` |
| Lanzador de KiUI | `~/.local/share/applications/dev.kitotsu.kiui.desktop` |
| Iconos de KiUI | `~/.local/share/icons/hicolor/` |
| Cache de artefactos | `~/.cache/gekkoapp/artifacts/` |
| Estado de GekkoApp | `~/.local/state/gekkoapp/installations-v1.json` |

Se respetan `XDG_BIN_HOME`, `XDG_DATA_HOME`, `XDG_CACHE_HOME` y
`XDG_STATE_HOME` cuando estan definidos. El `.desktop` ejecuta el entrypoint
absoluto de KiUI, por lo que no depende del `PATH` de la sesion grafica.

## Responsabilidades

GekkoApp instala dependencias del sistema, binarios, recursos, enlaces e
integraciones declaradas por los manifests. No crea ni inicia servicios de
wallpapers. KiUI y los CLI solicitan esas operaciones al Kitsune Compositor,
que decide como materializarlas para el escritorio y sistema compatibles.

## Estado actual

- Deteccion automatica y correccion manual: implementadas.
- Matriz de compatibilidad y bloqueo seguro: implementados.
- Seleccion modular con KiUI y compositor obligatorios: implementada.
- Resolucion de releases y manifests por target: implementada.
- Validacion de identidad, plataforma, glibc y dependencias: implementada.
- Descarga con limite, verificacion SHA-256 y cache: implementada.
- Extraccion segura sin `tar` externo y validacion del payload: implementada.
- Instalacion versionada, entrypoints, `.desktop`, iconos y estado: implementada.
- Sobrescritura de rutas ajenas: bloqueada.

Al 4 de agosto de 2026, Kitsune Compositor `v0.1.1`, KiUI `v0.1.1`, Kitowall
`v0.1.0` y Kilivepaper `v2.1.1` tienen releases compatibles. El preflight
conjunto de estos cuatro componentes esta validado. Kitsune todavia requiere su
propio release antes de poder seleccionarse.

## Seguridad

El arranque de GekkoApp ya no configura Chaotic AUR automaticamente. Las
operaciones que cambian el sistema solo se ejecutan desde opciones explicitas.
El preflight Kito no utiliza `sudo`. `sudo pacman` solo se ejecuta despues de
mostrar el plan y recibir confirmacion. Los tarballs rechazan rutas absolutas,
traversal, enlaces y archivos no declarados; ningun entrypoint o archivo de
escritorio ajeno se sobrescribe.
