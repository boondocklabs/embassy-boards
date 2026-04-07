//! DSI Display Initialization

use embassy_stm32::{
    dsihost::{
        DsiCommandConfig, DsiHost, DsiHostMode, DsiHostPhyConfig, DsiHostPhyLanes,
        DsiLtdcRefreshMode, DsiTearEventSource, panel::DsiPanel,
    },
    ltdc::{DSI, Ltdc, LtdcLayer, LtdcLayerConfig, PixelFormat, PolarityActive},
    peripherals,
};

use crate::{bsp::stm32::stm32h747i_disco::cm7::board::Buffers, display::glass::Glass};

pub(crate) async fn init_display(
    dsi: &mut DsiHost<'static, peripherals::DSIHOST>,
    ltdc: &mut Ltdc<'static, peripherals::LTDC, DSI>,
    buffers: &Buffers,
) {
    ltdc.init(&Glass::ltdc_config());

    // Disable LTDC while layers are initialized
    ltdc.disable();

    // Configure LTDC Layer 1 in ARGB8888 color mode with a rectangle covering panel active region
    let layer = LtdcLayerConfig {
        layer: LtdcLayer::Layer1,
        pixel_format: PixelFormat::ARGB8888,
        window_x0: 0,
        window_x1: Glass::ACTIVE_WIDTH,
        window_y0: 0,
        window_y1: Glass::ACTIVE_HEIGHT,
    };
    ltdc.init_layer(&layer, None);

    // Configure LTDC Layer 2 in ARGB8888 color mode with a rectangle covering panel active region
    let layer = LtdcLayerConfig {
        layer: LtdcLayer::Layer2,
        pixel_format: PixelFormat::ARGB8888,
        window_x0: 0,
        window_x1: Glass::ACTIVE_WIDTH,
        window_y0: 0,
        window_y1: Glass::ACTIVE_HEIGHT,
    };
    ltdc.init_layer(&layer, None);

    ltdc.enable();

    // Set the framebuffers for each layer
    ltdc.init_buffer(LtdcLayer::Layer2, buffers.fb0.ptr.as_ptr() as *const _);
    ltdc.init_buffer(LtdcLayer::Layer1, buffers.fb1.ptr.as_ptr() as *const _);

    // Reload the shadow registers
    ltdc.reload().await.unwrap();

    let mut command_config = DsiCommandConfig::default();
    command_config.refresh = DsiLtdcRefreshMode::Automatic;
    command_config.te_source = DsiTearEventSource::Gpio;
    command_config.te_polarity = PolarityActive::ActiveLow;

    // Initialize DSI PHY configuration struct
    let dsi_phy_config = DsiHostPhyConfig {
        lanes: DsiHostPhyLanes::Two,
        stop_wait_time: 10,
        acr: false,
        crc_rx: false,
        ecc_rx: false,
        eotp_rx: false,
        eotp_tx: false,
        bta: false,
        clock_hs2lp: 20,
        clock_lp2hs: 20,
        data_hs2lp: 20,
        data_lp2hs: 18,
        data_mrd: 0,
    };

    // Start the panel in Adapted Command mode
    dsi.start_panel::<Glass>(
        &dsi_phy_config,
        &DsiHostMode::AdaptedCommand(command_config),
    )
    .await
    .unwrap();
}
