use std::ffi::c_void;

pub struct RenderTexture {
    pub(crate) framebuffer_id: u32,
    pub(crate) rgb_texture_id: u32,
    pub(crate) depthbuffer_id: u32,
    pub(crate) depth_texture_id: u32,
    pub(crate) dimensions: (i32, i32),
}

impl RenderTexture {
    pub(crate) fn new() -> Self {
        Self {
            framebuffer_id: 0,
            rgb_texture_id: 0,
            depthbuffer_id: 0,
            depth_texture_id: 0,
            dimensions: (0, 0),
        }
    }

    pub(crate) fn init(&mut self, width: i32, height: i32) {
        unsafe {
            gl::GenFramebuffers(1, &mut self.framebuffer_id);
            gl::BindFramebuffer(gl::FRAMEBUFFER, self.framebuffer_id);
            
            // RGB texture
            gl::GenTextures(1, &mut self.rgb_texture_id);
            gl::BindTexture(gl::TEXTURE_2D, self.rgb_texture_id);
            gl::TexImage2D(gl::TEXTURE_2D, 0, gl::RGB as i32, width, height, 0, gl::RGB, gl::UNSIGNED_BYTE, 0 as *const std::ffi::c_void);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as i32);
            gl::FramebufferTexture2D(gl::FRAMEBUFFER, gl::COLOR_ATTACHMENT0, gl::TEXTURE_2D, self.rgb_texture_id, 0);

            // Depth buffer
            gl::GenRenderbuffers(1, &mut self.depthbuffer_id);
            gl::BindRenderbuffer(gl::RENDERBUFFER, self.depthbuffer_id);
            gl::RenderbufferStorage(gl::RENDERBUFFER, gl::DEPTH_COMPONENT, width, height);
            gl::FramebufferRenderbuffer(gl::FRAMEBUFFER, gl::DEPTH_ATTACHMENT, gl::RENDERBUFFER, self.depthbuffer_id);

            // Depth texture
            gl::GenTextures(1, &mut self.depth_texture_id);
            gl::BindTexture(gl::TEXTURE_2D, self.depth_texture_id);
            gl::TexImage2D(gl::TEXTURE_2D, 0, gl::DEPTH_COMPONENT as i32, width, height, 0, gl::DEPTH_COMPONENT, gl::FLOAT, 0 as *const std::ffi::c_void);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as i32); 
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_COMPARE_FUNC, gl::LEQUAL as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_COMPARE_MODE, gl::NONE as i32);
            gl::FramebufferTexture(gl::FRAMEBUFFER, gl::DEPTH_ATTACHMENT, self.depth_texture_id, 0);

            // Configure framebuffer
            
            //let draw_buffers: [u32; 2] = [gl::COLOR_ATTACHMENT0, gl::COLOR_ATTACHMENT1];
            //gl::DrawBuffers(2, &draw_buffers[0] as *const u32);

            if gl::CheckFramebufferStatus(gl::FRAMEBUFFER) != gl::FRAMEBUFFER_COMPLETE {
                //panic!("Could not setup framebuffer!")
            }
        }
        println!("Generated framebuffer {} with render texture {}", self.framebuffer_id, self.rgb_texture_id);
    }

    pub(crate) fn set_as_target(&self) {
        unsafe {
            gl::BindFramebuffer(gl::FRAMEBUFFER, self.framebuffer_id);
            gl::Viewport(0, 0, self.dimensions.0, self.dimensions.1);
        }
    }
}