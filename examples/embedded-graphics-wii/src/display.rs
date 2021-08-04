use core::{alloc::Layout, convert::TryInto, ffi::c_void, intrinsics::write_bytes};

use embedded_graphics::{
    draw_target::DrawTarget,
    pixelcolor::Rgb888,
    prelude::{OriginDimensions, RgbColor, Size},
    primitives::Rectangle,
    Pixel,
};
use ogc_rs::{
    ffi::{
        self as ogc_sys, GX_Color4u8, Mtx, GX_ALWAYS, GX_AOP_AND, GX_BL_INVSRCALPHA,
        GX_BL_SRCALPHA, GX_BM_BLEND, GX_CLIP_ENABLE, GX_CLR_RGBA, GX_COLOR0A0, GX_CULL_NONE,
        GX_DIRECT, GX_F32, GX_FALSE, GX_GM_1_0, GX_GREATER, GX_LEQUAL, GX_LO_CLEAR, GX_MAX_Z24,
        GX_NONE, GX_ORTHOGRAPHIC, GX_PASSCLR, GX_PF_RGB8_Z24, GX_PNMTX0, GX_POS_XYZ, GX_QUADS,
        GX_RGBA8, GX_TEVSTAGE0, GX_TEXCOORD0, GX_TEXMAP0, GX_TEX_ST, GX_TRUE, GX_VA_CLR0,
        GX_VA_POS, GX_VA_TEX0, GX_VTXFMT0, GX_ZC_LINEAR,
    },
    mem_cached_to_uncached,
    prelude::*,
};

pub struct Display;
impl Display {
    pub fn new(fifo_size: usize) -> Self {
        let buf: *mut c_void = unsafe {
            mem_cached_to_uncached!(alloc::alloc::alloc(
                Layout::from_size_align(fifo_size, 32).unwrap()
            )) as *mut c_void
        };
        unsafe {
            write_bytes(buf, 0, fifo_size);
        }
        Gx::init(buf, fifo_size as u32);
        Self
    }

    pub fn flush(&self, framebuffer: *mut c_void) {
        Gx::draw_done();
        Gx::set_z_mode(GX_TRUE as _, GX_LEQUAL as _, GX_TRUE as _);
        Gx::copy_disp(framebuffer, GX_TRUE as _);
    }

    pub fn setup(&self, rc: &mut RenderConfig) {
        let mut ident: Mtx = [[0.0; 4]; 3];
        Gx::set_copy_clear(Color::new(0, 0, 0, 0), GX_MAX_Z24);
        Gx::set_pixel_fmt(GX_PF_RGB8_Z24 as _, GX_ZC_LINEAR as _);
        Gx::set_viewport(
            0.0,
            0.0,
            rc.framebuffer_width as f32,
            rc.embed_framebuffer_height as f32,
            0.0,
            0.0,
        );

        let yscale =
            Gx::get_y_scale_factor(rc.embed_framebuffer_height, rc.extern_framebuffer_height);
        let extern_framebuffer_height = Gx::set_disp_copy_y_scale(yscale) as u16;

        let mut half_aspect_ratio = GX_FALSE;
        if rc.vi_height == 2 * rc.extern_framebuffer_height {
            half_aspect_ratio = GX_TRUE
        }

        Gx::set_disp_copy_src(0, 0, rc.framebuffer_width, rc.embed_framebuffer_height);
        Gx::set_disp_copy_dst(rc.framebuffer_width, extern_framebuffer_height);
        Gx::set_copy_filter(
            rc.anti_aliasing,
            &mut rc.sample_pattern,
            GX_TRUE as _,
            &mut rc.v_filter,
        );
        Gx::set_field_mode(rc.field_rendering, half_aspect_ratio as _);
        Gx::set_disp_copy_gamma(GX_GM_1_0 as _);

        //Clear VTX
        Gx::clear_vtx_desc();
        Gx::inv_vtx_cache();
        Gx::invalidate_tex_all();

        Gx::set_vtx_desc(GX_VA_TEX0 as _, GX_NONE as _);
        Gx::set_vtx_desc(GX_VA_POS as _, GX_DIRECT as _);
        Gx::set_vtx_desc(GX_VA_CLR0 as _, GX_DIRECT as _);

        Gx::set_vtx_attr_fmt(
            GX_VTXFMT0 as _,
            GX_VA_POS as _,
            GX_POS_XYZ as _,
            GX_F32 as _,
            0,
        );
        Gx::set_vtx_attr_fmt(GX_VTXFMT0 as _, GX_VA_TEX0, GX_TEX_ST as _, GX_F32 as _, 0);
        Gx::set_vtx_attr_fmt(
            GX_VTXFMT0 as _,
            GX_VA_CLR0,
            GX_CLR_RGBA as _,
            GX_RGBA8 as _,
            0,
        );
        Gx::set_z_mode(GX_TRUE as _, GX_LEQUAL as _, GX_TRUE as _);

        Gx::set_num_chans(1);
        Gx::set_num_tex_gens(1);
        Gx::set_tev_op(GX_TEVSTAGE0 as _, GX_PASSCLR as _);
        Gx::set_tev_order(
            GX_TEVSTAGE0 as _,
            GX_TEXCOORD0 as _,
            GX_TEXMAP0 as _,
            GX_COLOR0A0 as _,
        );
        Gu::mtx_identity(&mut ident);
        Gu::mtx_trans_apply(&mut ident.clone(), &mut ident, 0.0, 0.0, -100.0);
        Gx::load_pos_mtx_imm(&mut ident, GX_PNMTX0 as _);

        let mut perspective: ogc_rs::ffi::Mtx44 = [[0f32; 4]; 4];

        Gu::ortho(
            &mut perspective,
            0.0,
            rc.embed_framebuffer_height as f32,
            0.0,
            rc.framebuffer_width as f32,
            0.0,
            1000.0,
        );
        Gx::load_projection_mtx(&mut perspective, GX_ORTHOGRAPHIC as _);

        Gx::set_viewport(
            0.0,
            0.0,
            rc.framebuffer_width as f32,
            rc.embed_framebuffer_height as f32,
            0.0,
            1.0,
        );
        Gx::set_blend_mode(
            GX_BM_BLEND as _,
            GX_BL_SRCALPHA as _,
            GX_BL_INVSRCALPHA as _,
            GX_LO_CLEAR as _,
        );
        Gx::set_alpha_update(GX_TRUE as _);
        Gx::set_alpha_compare(GX_GREATER as _, 0, GX_AOP_AND as _, GX_ALWAYS as _, 0);
        Gx::set_color_update(GX_TRUE as _);
        Gx::set_cull_mode(GX_CULL_NONE as _);

        Gx::set_clip_mode(GX_CLIP_ENABLE as _);
        Gx::set_scissor(
            0,
            0,
            rc.framebuffer_width.into(),
            rc.embed_framebuffer_height.into(),
        );
    }
}

impl DrawTarget for Display {
    type Color = Rgb888;
    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = embedded_graphics::Pixel<Self::Color>>,
    {
        for Pixel(coord, color) in pixels.into_iter() {
            if let Ok((x @ 0..=639, y @ 0..=527)) = coord.try_into() {
                let poke_x: u32 = x;
                let poke_y: u32 = y;
                Gx::poke_argb(
                    poke_x as u16,
                    poke_y as u16,
                    Color::new(color.r(), color.g(), color.b(), 255),
                )
            }
        }

        Ok(())
    }

    fn fill_solid(&mut self, area: &Rectangle, color: Self::Color) -> Result<(), Self::Error> {
        Gx::begin(GX_QUADS as _, GX_VTXFMT0 as _, 4);
        Gx::position_3f32(area.top_left.x as _, area.top_left.y as _, 0.0);
        unsafe {
            GX_Color4u8(color.r(), color.g(), color.b(), 255);
        }
        Gx::position_3f32(
            area.bottom_right().unwrap().x as _,
            area.top_left.y as _,
            0.0,
        );
        unsafe {
            GX_Color4u8(color.r(), color.g(), color.b(), 255);
        }
        Gx::position_3f32(
            area.bottom_right().unwrap().x as _,
            area.bottom_right().unwrap().y as _,
            0.0,
        );
        unsafe {
            GX_Color4u8(color.r(), color.g(), color.b(), 255);
        }
        Gx::position_3f32(
            area.top_left.x as _,
            area.bottom_right().unwrap().y as _,
            0.0,
        );
        unsafe {
            GX_Color4u8(color.r(), color.g(), color.b(), 255);
        }
        Gx::end();

        Ok(())
    }
}

impl OriginDimensions for Display {
    fn size(&self) -> Size {
        Size::new(640, 528)
    }
}
