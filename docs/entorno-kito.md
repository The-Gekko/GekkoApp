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
4. El usuario selecciona Kitowall, Kilivepaper y/o KiSDDM. Kitsune se muestra
   como proximamente y no puede agregarse al plan.
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

## Matriz soportada

El gate de entorno vive en `environment.rs::refresh_compatibility` (es el que
decide si la tarjeta de Kito esta disponible):

| Dimension | Requisito | ¿Bloquea la instalacion? |
| --- | --- | --- |
| Distribucion | Arch Linux (o `ID_LIKE=arch`) o Solus | Si |
| Arquitectura | x86_64 | Si |
| Servicios | systemd de usuario | Si |
| Sesion | Wayland recomendada | No |
| Escritorio | Hyprland recomendado | No |
| Target | x86_64-unknown-linux-gnu | Si |

La sesion y el escritorio se detectan y se muestran en el diagnostico, pero ya
no bloquean la instalacion: el requisito de Wayland + Hyprland se retiro. La
deteccion de Ubuntu/Debian, Fedora, GNOME y KDE existe para dar un diagnostico
correcto, pero esas distribuciones siguen sin estar soportadas.

### Dependencias de host por distribucion

`InstallationPlan::required_host_packages` traduce las capacidades obligatorias
que declara cada manifest al gestor de paquetes de la distribucion:

| Capacidad | Arch | Solus |
| --- | --- | --- |
| `runtime.qt6` | `qt6-base`, `qt6-declarative`, `qt6-wayland` | iguales |
| `gpu.wgpu` | `vulkan-icd-loader`, `wayland`, `libxkbcommon` | `vulkan`, `wayland`, `libxkbcommon` |
| `audio.pipewire` | `pipewire` | `pipewire` |
| `renderer.awww` | `awww` | sin equivalente: se aborta antes de tocar nada |

Ese mapeo es un **segundo** gate, posterior al de entorno: `install_kito_plan`
aborta antes de descargar o instalar nada si alguna capacidad obligatoria del
manifiesto no tiene paquete en la distribucion actual.
| `session.wayland` | sin paquete | sin paquete |

## Repositorios resueltos

| Componente | Repositorio | Tipo |
| --- | --- | --- |
| KiUI | KitotsuMolina/KiUI | obligatorio |
| Kitsune Compositor | KitotsuMolina/Kito-compositor | obligatorio |
| Kitowall | KitotsuMolina/KitowallV2 | seleccionable |
| Kilivepaper | KitotsuMolina/Kilivepaper | seleccionable |
| KiSDDM | KitotsuMolina/KiSDDM | seleccionable |
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
absoluto de KiUI y le entrega mediante variables `KIUI_*_BIN` las rutas absolutas
de los CLI del ecosistema. Por ello ni KiUI ni sus modulos dependen del `PATH` de
la sesion grafica.

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

Kitsune Compositor, KiUI, Kitowall, Kilivepaper y KiSDDM publican releases
compatibles con este contrato. GekkoApp resuelve sus versiones estables más
recientes durante el preflight, sin fijarlas en el código. Kitsune todavía
requiere su propio release antes de poder seleccionarse.

## Seguridad

El arranque de GekkoApp ya no configura Chaotic AUR automaticamente. Las
operaciones que cambian el sistema solo se ejecutan desde opciones explicitas.
El preflight Kito no utiliza `sudo`. `sudo pacman` solo se ejecuta despues de
mostrar el plan y recibir confirmacion. Los tarballs rechazan rutas absolutas,
traversal, enlaces y archivos no declarados; ningun entrypoint o archivo de
escritorio ajeno se sobrescribe.
