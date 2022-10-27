# tt

This project contains a port of [egui](https://github.com/emilk/egui) to the Spotify Car Thing although in the future I also plan for it to contain a custom Car Thing UI.

![](branding/demo.jpg)

## Technical information

- Display/Touchscreen:
  - It's actually a portrait touchscreen
    - Seems like no way to rotate in hardware:
      - `/sys/class/graphics/fb0/rotate` doesn't seem to work
      - `/sys/class/graphics/fb0/osd_reverse` only flips/mirrors it (so it doesn't help either)
      - `echo 1 > /sys/class/graphics/fb0/osd_clear` can be used to clear the display
- A [branch of winit with KMS/DRM](https://github.com/rust-windowing/winit/pull/2272) is used:
  - The KMS/DRM portion of the branch is not actually being used because the Car Thing doesn't support KMS/DRM.
    - The Car Thing supports FBDev instead (but I don't actually use that either, I use EGL directly)
  - Instead, it is being used because it supports libinput.
  - Other changes:
      - needed to config a custom calibration matrix in libinput to rotate the touchscreen
      - libinput `Touch::Frame` event is ignored (in the original implementation it was mapped to TouchEnd which doesn't make sense, that's not what it means)
      - `xkb_compose` is disabled, it caused runtime errors and we don't need it on the Car Thing anyways cause it doesn't have a keyboard
- I have forked egui:
  - `egui_glow` rendering backend is used
    - OpenGL shader had to be modified to rotate everything (cause the touchscreen is rotated)
    - Calls to `glViewport` and `glScissor` has to be modified to be rotated as well
    - Custom rendering is still broken, I'm not sure how to fix that as I don't think there is a way to tell OpenGL to rotate everything rendered by custom renderers.
      - Maybe have custom renderers render to an intermediate buffer and then rotate that when copying it to the output framebuffer?
- I have forked glutin:
  - A modification of the Android backend is used because it is similar to the graphical config of the Car Thing
  - KMS/DRM support is **NOT** used because again, the Car Thing is more similar to Android in that respect
- `buildroot.sh` builds and deploys the program using [my Car Thing buildroot](https://github.com/null-dev/car-thing-buildroot)
  - You will probably have to modify it so it uses the correct paths and `adb`