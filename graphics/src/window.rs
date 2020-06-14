use vulkano::{
    command_buffer::{AutoCommandBufferBuilder, DynamicState},
    device::Device,
    framebuffer::RenderPassAbstract,
    image::swapchain::SwapchainImage,
    instance::Instance,
    swapchain,
    swapchain::{
        AcquireError,
        FullscreenExclusive,
        PresentMode,
        Surface,
        SurfaceTransform,
        Swapchain,
        SwapchainAcquireFuture,
        SwapchainCreationError,
    },
    sync,
    sync::{FlushError, GpuFuture},
};

use vulkano_win::VkSurfaceBuild;
use winit::{
    event_loop::EventLoopWindowTarget,
    window::{Window, WindowBuilder, WindowId},
};

use std::sync::Arc;

use crate::{
    common::*,
    config,
    error::RenderingError,
    geometry::Geometry,
    renderer::Renderer,
    target::RenderTarget,
};

use polyengine_core::log;

pub struct WindowContext {
    device: Arc<Device>,
    queue: Arc<vulkano::device::Queue>,
    pub surface: Arc<Surface<Window>>,
    pub swapchain: Arc<Swapchain<Window>>,
    pub images: Vec<Arc<SwapchainImage<Window>>>,
    pub render_target: RenderTarget,

    pub dynamic_state: DynamicState,
    pub recreate_swapchain: bool,
    pub previous_frame_end: Option<Box<dyn GpuFuture>>,
}

impl WindowContext {
    pub fn new(
        elwt: &EventLoopWindowTarget<()>,
        instance: Arc<Instance>,
        device: Arc<Device>,
        queue: Arc<vulkano::device::Queue>,
        default_window_render_pass: Arc<dyn RenderPassAbstract + Send + Sync>,
    ) -> Self {
        let surface = WindowBuilder::new()
            .build_vk_surface(&elwt, instance.clone())
            .unwrap();

        let (swapchain, images) = {
            // Querying the capabilities of the surface. When we create the swapchain we can
            // only pass values that are allowed by the capabilities.
            let caps = surface.capabilities(device.physical_device()).unwrap();
            let usage = caps.supported_usage_flags;
            let dimensions: [u32; 2] = surface.window().inner_size().into();

            let check_default_format_available = |caps: &vulkano::swapchain::Capabilities| -> bool {
                for (format, color_space) in &caps.supported_formats {
                    if config::DEFAULT_WINDOW_FORMAT == *format
                        && config::DEFAULT_COLOR_SPACE == *color_space
                    {
                        return true;
                    }
                }
                return false;
            };
            assert!(check_default_format_available(&caps));

            // Please take a look at the docs for the meaning of the parameters we didn't
            // mention.
            Swapchain::new(
                device.clone(),
                surface.clone(),
                caps.min_image_count,
                config::DEFAULT_WINDOW_FORMAT,
                dimensions,
                1,
                usage,
                &queue,
                SurfaceTransform::Identity,
                config::DEFAULT_WINDOW_ALPHA,
                PresentMode::Fifo,
                FullscreenExclusive::Default,
                true,
                config::DEFAULT_COLOR_SPACE,
            )
            .unwrap()
        };

        let mut dynamic_state = DynamicState {
            line_width: None,
            viewports: None,
            scissors: None,
            compare_mask: None,
            write_mask: None,
            reference: None,
        };
        let framebuffers = window_size_dependent_setup(
            &images,
            default_window_render_pass.clone(),
            &mut dynamic_state,
        );
        let recreate_swapchain = false;
        let previous_frame_end = Some(Box::new(sync::now(device.clone())) as Box<dyn GpuFuture>);

        let render_target = RenderTarget {
            render_pass: default_window_render_pass.clone(),
            framebuffers,
        };

        return WindowContext {
            device,
            queue,
            surface,
            swapchain,
            images,
            render_target,
            dynamic_state,
            recreate_swapchain,
            previous_frame_end,
        };
    }

    pub fn id(&self) -> WindowId { return self.surface.window().id(); }

    pub fn on_resize(&mut self) { self.recreate_swapchain = true; }

    pub fn acquire_next_image(
        &mut self,
    ) -> Result<(usize, SwapchainAcquireFuture<Window>), RenderingError> {
        self.previous_frame_end.as_mut().unwrap().cleanup_finished();

        // Whenever the window resizes we need to recreate everything dependent on the
        // window size. In this example that includes the swapchain, the
        // framebuffers and the dynamic state viewport.
        if self.recreate_swapchain {
            // Get the new dimensions of the window.
            let dimensions: [u32; 2] = self.surface.window().inner_size().into();
            let (new_swapchain, new_images) =
                match self.swapchain.recreate_with_dimensions(dimensions) {
                    Ok(r) => r,
                    // This error tends to happen when the user is manually resizing the window.
                    // Simply restarting the loop is the easiest way to fix this issue.
                    Err(SwapchainCreationError::UnsupportedDimensions) => {
                        return Err(RenderingError::RecreateSwapchainFailed);
                    }
                    Err(e) => panic!("Failed to recreate swapchain: {:?}", e),
                };

            self.swapchain = new_swapchain;
            // Because framebuffers contains an Arc on the old swapchain, we need to
            // recreate framebuffers as well.
            self.render_target.framebuffers = window_size_dependent_setup(
                &new_images,
                self.render_target.render_pass.clone(),
                &mut self.dynamic_state,
            );
            self.recreate_swapchain = false;
        }

        // Before we can draw on the output, we have to *acquire* an image from the
        // swapchain. If no image is available (which happens if you submit draw
        // commands too quickly), then the function will block.
        // This operation returns the index of the image that we are allowed to draw
        // upon.
        //
        // This function can block if no image is available. The parameter is an
        // optional timeout after which the function call will return an error.
        let (image_num, suboptimal, acquire_future) =
            match swapchain::acquire_next_image(self.swapchain.clone(), None) {
                Ok(r) => r,
                Err(AcquireError::OutOfDate) => {
                    self.recreate_swapchain = true;
                    return Err(RenderingError::ImageAcquireFailed);
                }
                Err(e) => panic!("Failed to acquire next image: {:?}", e),
            };

        // acquire_next_image can be successful, but suboptimal. This means that the
        // swapchain image will still work, but it may not display correctly.
        // With some drivers this can be when the window resizes, but it may not
        // cause the swapchain to become out of date.
        if suboptimal {
            self.recreate_swapchain = true;
        }

        return Ok((image_num, acquire_future));
    }

    pub fn draw(
        &mut self,
        image_num: usize,
        acquire_future: SwapchainAcquireFuture<Window>,
        renderer: &Renderer,
        geometry: &Geometry,
    ) {
        // Specify the color to clear the framebuffer with i.e. blue
        let clear_values = vec![[0.0, 0.0, 1.0, 1.0].into()];

        let command_buffer = AutoCommandBufferBuilder::primary_one_time_submit(
            self.device.clone(),
            self.queue.family(),
        )
        .unwrap()
        .begin_render_pass(
            self.render_target.framebuffers[image_num].clone(),
            false,
            clear_values,
        )
        .unwrap()
        .draw(
            renderer.pipeline.clone(),
            &self.dynamic_state,
            geometry.vertex_buffer.clone(),
            (),
            (),
        )
        .unwrap()
        .end_render_pass()
        .unwrap()
        .build()
        .unwrap();

        let future = self
            .previous_frame_end
            .take()
            .unwrap()
            .join(acquire_future)
            .then_execute(self.queue.clone(), command_buffer)
            .unwrap()
            .then_swapchain_present(self.queue.clone(), self.swapchain.clone(), image_num)
            .then_signal_fence_and_flush();

        match future {
            Ok(future) => {
                self.previous_frame_end = Some(Box::new(future) as Box<_>);
            }
            Err(FlushError::OutOfDate) => {
                self.recreate_swapchain = true;
                self.previous_frame_end = Some(Box::new(sync::now(self.device.clone())) as Box<_>);
            }
            Err(e) => {
                log::error!("Failed to flush future: {:?}", e);
                self.previous_frame_end = Some(Box::new(sync::now(self.device.clone())) as Box<_>);
            }
        }
    }
}
