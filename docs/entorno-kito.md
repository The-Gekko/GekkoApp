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
4. El usuario puede seleccionar Kitowall, Kilivepaper y/o KiSDDM, o continuar
   sin modulos opcionales. KiSDDM requiere SDDM instalado. Kitsune se muestra
   como proximamente y no puede agregarse al plan.
5. KiUI se agrega siempre al plan junto con Kitsune Compositor, su dependencia
   tecnica (no es el compositor de escritorio Niri o Hyprland).
6. GekkoApp consulta el release estable mas reciente de cada repositorio.
7. Descarga y valida todos los manifests, incluida la plataforma, glibc minima,
   dependencias modulares, payload e integraciones de escritorio.
8. Si falta un artefacto para el target detectado, se aborta antes de modificar
   archivos o instalar paquetes.
9. Presenta el plan y solicita confirmacion explicita.
10. Descarga todos los artefactos y verifica tamano y SHA-256 antes de instalar
    paquetes del sistema.
11. Instala las dependencias de host faltantes mediante `pacman`; despues extrae
    los artefactos en staging y valida cada archivo contra el manifest.
12. Activa los entrypoints y la integracion de escritorio, y registra el estado.

## Matriz soportada en la primera version

| Dimension | Soporte |
| --- | --- |
| Distribucion | Arch Linux y derivadas |
| Arquitectura | x86_64 |
| Sesion | Wayland |
| Escritorio | Hyprland o Niri |
| Servicios | systemd de usuario |
| Target | x86_64-unknown-linux-gnu |

La deteccion de Ubuntu/Debian, Fedora, GNOME y KDE existe para generar un
diagnostico correcto, pero todavia no habilita su instalacion.

## Repositorios resueltos

| Componente | Repositorio | Tipo |
| --- | --- | --- |
| KiUI | KitotsuMolina/KiUI | obligatorio |
| Kitsune Compositor | KitotsuMolina/Kito-compositor | dependencia tecnica de KiUI |
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

Actualizacion 2026-09-25: la deteccion distingue compositor, shells instalados,
gestor de login configurado y greeter. Niri y Hyprland estan habilitados.
DMS y Caelestia se muestran por disponibilidad; no se asegura que su IPC este
ejecutandose. El flujo Kito no instala Niri, Hyprland, DMS ni Caelestia, y no
modifica sus configuraciones.

KiSDDM es opcional y solo se habilita cuando existe el binario `sddm` en PATH o
en `/usr/bin/sddm`. No basta un enlace de servicio configurado. SDDM puede estar
instalado aunque greetd sea el gestor configurado: ese caso permite seleccionar
KiSDDM, sin activar SDDM ni cambiar el gestor de login. GekkoApp no instala SDDM.
La integracion para personalizar DMS greeter queda pendiente.

KiUI es el unico modulo de producto obligatorio; Kitsune Compositor se incluye
como dependencia tecnica. Todos los opcionales empiezan desmarcados. Kitsune
(espectro de audio) sigue deshabilitado y no se modifica.

### Dependencias y versiones

El preflight interpreta `requirements.modules[].constraint` como rangos SemVer.
Exige las dependencias obligatorias y verifica sus versiones. Un modulo opcional
ausente no se instala automaticamente; si esta en el plan, tambien debe cumplir
el rango. Versiones o restricciones malformadas abortan el plan. Se comparan los
releases seleccionados; no se reconcilian aqui los modulos de instalaciones
anteriores que no formen parte del plan. Desmarcar un modulo no lo desinstala.

La resolucion sigue usando el ultimo release estable de cada repositorio. Si
sus versiones no son compatibles, se informa del error; no se busca un release
antiguo automaticamente.

Los paquetes se derivan de las capacidades obligatorias de los manifests:

| Capacidad | Paquetes Arch |
| --- | --- |
| `runtime.qt6` | qt6-base, qt6-declarative, qt6-imageformats, qt6-wayland |
| `renderer.awww` | awww |
| `gpu.wgpu` | vulkan-icd-loader, wayland, libxkbcommon |
| `audio.pipewire` | pipewire |

`qt6-imageformats` permite cargar miniaturas WebP. La lista mostrada es la de
dependencias requeridas; antes de instalar, `pacman -Qq` filtra las ya instaladas.
Solo las faltantes pasan a `sudo pacman -S --needed`. Las capacidades opcionales
no fuerzan paquetes. La instalacion requiere que los paquetes esten disponibles
en los repositorios configurados; no agrega repositorios automaticamente.

Los presets generales de GekkoApp permanecen separados de la opcion Kito; sus
operaciones de instalacion del escritorio no forman parte de este flujo.

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
