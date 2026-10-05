# Kito en GekkoApp Control Center

Actualizado el 25 de septiembre de 2026. Esta integración parte de la arquitectura
Tauri de main (GekkoApp 1.3.0), con lógica Rust compartida por GUI y CLI.

## Alcance

GekkoApp instala los componentes Kito seleccionados y sus dependencias faltantes.
No instala ni configura Niri, Hyprland, DMS, Caelestia o el gestor de login.
Kitsune (espectro de audio) es un módulo opcional independiente de Kito Compositor.

KiUI es el producto obligatorio. Kito Compositor se incluye como dependencia
técnica de KiUI; no sustituye al compositor de escritorio. Kitowall, Kilivepaper
KiSDDM y Kitsune son opcionales. GUI y CLI permiten instalar únicamente la base.

## Detección y disponibilidad

La compatibilidad general de GekkoApp conserva Arch/Solus y derivadas, x86_64 y
systemd. Los requisitos específicos de Kito se comprueban aparte: sesión Wayland
y compositor Niri o Hyprland. Esto no bloquea el catálogo de otras aplicaciones.

Se distinguen:
- Compositor de escritorio, a partir de las variables de sesión.
- Shells disponibles en PATH: DMS y Caelestia; esto no demuestra que su IPC esté activo.
- Gestor de login configurado por el enlace display-manager.service.
- Greeter: reconoce la orden simple dms-greeter en la sesión predeterminada de greetd.
- SDDM instalado: binario sddm en PATH o /usr/bin/sddm.

KiSDDM requiere SDDM instalado, aunque el gestor configurado sea greetd. Un enlace
sddm.service sin el binario no basta. La GUI muestra el motivo y deshabilita su
casilla; la lógica Rust rechaza también una selección inválida. No se instala ni
activa SDDM automáticamente. Personalizar DMS greeter queda pendiente.

## Flujo gráfico

1. El catálogo muestra el entorno, la base y los módulos opcionales. Los módulos
   opcionales ya instalados aparecen marcados si están disponibles; pueden desmarcarse.
2. Instalar / Actualizar prepara el plan sin instalar paquetes ni activar componentes.
3. Se resuelven releases estables y se validan manifiestos, plataforma, glibc,
   identidad, dependencias y rangos SemVer.
4. Se calculan paquetes requeridos y cuáles faltan. Si faltan paquetes, se pide
   completar el campo de contraseña de sudo antes de continuar.
5. El modal presenta componentes con versiones exactas, dependencias requeridas
   y paquetes faltantes. Cancelar descarta el plan.
6. Confirmar consume una sola vez el plan preparado. No se vuelve a resolver
   latest: se usan los mismos manifiestos revisados. Se revalida el entorno.
7. Se descargan y verifican todos los artefactos antes de instalar paquetes.
8. Se instalan solo dependencias faltantes, se validan y activan los artefactos
   y se actualiza el estado del instalador.

El plan reside en memoria y tiene un identificador. Preparar otro plan reemplaza
el anterior; un identificador inválido o ya consumido no instala nada. No se
necesita contraseña para Kito si todas las dependencias ya están instaladas.
La CLI conserva su confirmación y el prompt normal de sudo.

Desmarcar un módulo no lo desinstala. La desinstalación sigue siendo una acción
separada del Control Center. Se conserva el motor existente de instalaciones
versionadas, rutas XDG, actualización y limpieza de archivos administrados.

## Dependencias y versiones

Los rangos requirements.modules[].constraint son obligatorios y se interpretan
con SemVer. Una dependencia requerida ausente bloquea el plan. Una opcional
ausente no se instala automáticamente; si está seleccionada, su versión también
debe cumplir el rango. Las restricciones o versiones inválidas se rechazan.
La identidad de repositorio se compara sin distinguir mayúsculas, preservando
la lista existente de alias permitidos.

Se comparan los releases del plan; no se reconcilian módulos antiguos que no
estén seleccionados. Si los últimos releases son incompatibles, se informa del
error; no se seleccionan versiones antiguas automáticamente.

En Arch, runtime.qt6 incorpora qt6-imageformats para miniaturas WebP, además de
qt6-base, qt6-declarative y qt6-wayland. Se conservan los mapeos de awww, Vulkan,
Wayland, libxkbcommon y PipeWire según las capacidades obligatorias del plan.
La instalación filtra los paquetes ya presentes.

Solus conserva su mapeo de paquetes anterior. No se ha certificado aquí su
combinación Niri/DMS ni el paquete de formatos Qt; awww sigue sin mapeo en Solus,
por lo que un plan que lo requiera se rechaza antes de instalar. Los paquetes
deben estar disponibles en los repositorios del usuario; no se agregan
repositorios automáticamente.

## Código y verificación

- src/environment.rs: detección y requisitos específicos de Kito.
- src/core/flow.rs: preparación e instalación compartidas por GUI y CLI.
- src/gui/mod.rs: catálogo, plan pendiente y comandos Tauri.
- src/installer.rs: dependencias, manifiestos y motor de instalación.
- ui/app.js y ui/styles.css: disponibilidad y confirmación del plan.

Comprobaciones:

    cargo fmt --all -- --check
    cargo test --locked --all-targets --all-features
    cargo clippy --locked --all-targets --all-features -- -D warnings
    node --test tests/kito-ui.test.cjs

Las pruebas cubren Niri/greetd, SDDM instalado frente a configurado, separación
entre compatibilidad global y Kito, selección vacía, SemVer, consumo único de
planes y confirmación/cancelación de la GUI. No se realiza una instalación real
sobre el equipo durante estas pruebas.
