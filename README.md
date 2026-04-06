## Embassy Board Support

A work in progress framework and collection of board support packages for Embassy.

* Trait based board definitions
* Const memory layout definition, with automatic memory.x generation
  * MPU attributes specified on memory layout automatically configured on Cortex-M7
* Provides board specific Devices struct from init with initialized drivers
* DMA2D text rendering engine with native Ratatui backend
* Two layer alpha blended LTDC frame buffers
  * Render a Ratatui terminal on top of a background
* Dual core STM32H7 support

