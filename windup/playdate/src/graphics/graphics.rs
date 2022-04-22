use core::ffi::c_void;

use super::active_font::ActiveFont;
use super::bitmap::{Bitmap, BitmapRef, SharedBitmapRef};
use super::bitmap_collider::BitmapCollider;
use super::color::Color;
use super::font::Font;
use super::framebuffer_stencil_bitmap::FramebufferStencilBitmap;
use crate::capi_state::{CApiState, ContextStackId};
use crate::ctypes::*;
use crate::null_terminated::ToNullTerminatedString;

/// Access to drawing functions to draw to the Playdate device's screen.
#[derive(Debug)]
#[non_exhaustive]
pub struct Graphics;
impl Graphics {
  pub(crate) fn new() -> Self {
    Graphics
  }

  pub fn bitmaps_collide(
    &self,
    a: BitmapCollider,
    b: BitmapCollider,
    in_rect: euclid::default::Rect<i32>,
  ) -> bool {
    unsafe {
      // checkMaskCollision expects `*mut CLCDBitmap` but it only reads from the bitmaps to check
      // for collision, so we can cast from a shared reference on Bitmap to a mut pointer.
      CApiState::get().cgraphics.checkMaskCollision.unwrap()(
        a.bitmap.as_bitmap_ptr(),
        a.x,
        a.y,
        a.flipped,
        b.bitmap.as_bitmap_ptr(),
        b.x,
        b.y,
        b.flipped,
        super::playdate_rect_from_euclid(in_rect),
      ) != 0
    }
  }

  /// Clears the entire display, filling it with `color`.
  pub fn clear<'a, C>(&mut self, color: C)
  where
    Color<'a>: From<C>,
  {
    unsafe {
      CApiState::get().cgraphics.clear.unwrap()(Color::<'a>::from(color).to_c_color());
    }
  }

  /// Sets the background color shown when the display is offset or for clearing dirty areas
  /// in the sprite system.
  pub fn set_background_color(&mut self, color: SolidColor) {
    unsafe {
      CApiState::get().cgraphics.setBackgroundColor.unwrap()(color);
    }
  }

  /// Manually flushes the current frame buffer out to the display. This function is automatically
  /// called at the end of each frame, after yielding back to the Playdate system through the
  /// `SystemEventWatcher`, so there shouldn’t be any need to call it yourself.
  pub fn display(&mut self) {
    unsafe {
      CApiState::get().cgraphics.display.unwrap()();
    }
  }

  /// Returns the debug framebuffer as a bitmap.
  ///
  /// Only valid in the simulator, so not compiled for device builds.
  #[cfg(not(all(target_arch = "arm", target_os = "none")))]
  pub fn debug_frame_bitmap(&self) -> SharedBitmapRef<'static> {
    let bitmap_ptr = unsafe { CApiState::get().cgraphics.getDebugBitmap.unwrap()() };
    assert!(!bitmap_ptr.is_null());
    SharedBitmapRef::from_ptr(bitmap_ptr)
  }

  /// Returns a copy of the contents of the display front buffer.
  ///
  /// The Playdate device is double-buffered, and this returns the currently displayed frame.
  pub fn display_frame_bitmap(&self) -> Bitmap {
    let bitmap_ptr = unsafe { CApiState::get().cgraphics.getDisplayBufferBitmap.unwrap()() };
    use alloc::borrow::ToOwned;
    BitmapRef::from_ptr(bitmap_ptr).to_owned()
  }

  /// Returns a copy the contents of the working frame buffer as a bitmap.
  ///
  /// The Playdate device is double-buffered, and this returns the buffer that will be displayed
  /// next frame.
  pub fn working_frame_bitmap(&self) -> Bitmap {
    let bitmap_ptr = unsafe { CApiState::get().cgraphics.copyFrameBufferBitmap.unwrap()() };
    Bitmap::from_owned_ptr(bitmap_ptr)
  }

  /// After updating pixels in the buffer returned by `get_frame()`, you must tell the graphics
  /// system which rows were updated. This function marks a contiguous range of rows as updated
  /// (e.g., `mark_updated_rows(0, LCD_ROWS - 1)` tells the system to update the entire display).
  /// Both "start" and "end" are included in the range.
  pub fn mark_updated_rows(&mut self, start: i32, end: i32) {
    unsafe { CApiState::get().cgraphics.markUpdatedRows.unwrap()(start, end) }
  }

  /// Offsets the origin point for all drawing calls to x, y (can be negative).
  pub fn set_draw_offset(&mut self, dx: i32, dy: i32) {
    unsafe { CApiState::get().cgraphics.setDrawOffset.unwrap()(dx, dy) }
  }

  /// Push a new drawing context that targets the display framebuffer.
  ///
  /// Drawing functions use a context stack to select the drawing target, for setting a stencil,
  /// changing the draw mode, etc. The stack is unwound at the beginning of each update cycle, with
  /// drawing restored to target the display framebuffer.
  pub fn push_context(&mut self) {
    CApiState::get().stack.borrow_mut().push_framebuffer()
  }
  /// Push a drawing context that targets a bitmap.
  ///
  /// Drawing functions use a context stack to select the drawing target, for setting a stencil,
  /// changing the draw mode, etc. The stack is unwound at the beginning of each update cycle, with
  /// drawing restored to target the display framebuffer.
  ///
  /// When the bitmap's drawing is popped, either by calling pop_context() or at the end of the
  /// frame, it will be kept alive as long as the ContextStackId returned here (or a clone of it) is
  /// kept alive.
  pub fn push_context_bitmap(&mut self, bitmap: Bitmap) -> ContextStackId {
    CApiState::get().stack.borrow_mut().push_bitmap(bitmap)
  }
  /// Pop the top (most recently pushed, and not yet popped) drawing context from the stack.
  ///
  /// Drawing functions use a context stack to select the drawing target, for setting a stencil,
  /// changing the draw mode, etc. The stack is unwound at the beginning of each update cycle, with
  /// drawing restored to target the display framebuffer.
  ///
  /// The returned ContextStackId, if present, can be used to get back the Bitmap that was drawn
  /// into for the popped drawing context. A ContextStackId is not returned if the popped drawing
  /// context was drawing into the display framebuffer.
  pub fn pop_context(&mut self) -> Option<ContextStackId> {
    CApiState::get().stack.borrow_mut().pop(CApiState::get())
  }
  /// Retrieve an Bitmap that was pushed into a drawing context with push_context_bitmap() and
  /// since popped off the stack, either with pop_context() or at the end of the frame.
  pub fn take_popped_context_bitmap(&mut self, id: ContextStackId) -> Option<Bitmap> {
    CApiState::get().stack.borrow_mut().take_bitmap(id)
  }

  /// Sets the stencil used for drawing.
  ///
  /// If the image is smaller than full screen, its width should be a multiple of 32 pixels.
  /// Stencils smaller than full screen will be tiled.
  ///
  /// The bitmap will remain the stencil as long as the FramebufferStencilBitmap is not dropped, or another
  /// call to set_stencil() is made.
  pub fn set_stencil<'a>(&mut self, bitmap: &'a BitmapRef) -> FramebufferStencilBitmap<'a> {
    unsafe { CApiState::get().cgraphics.setStencil.unwrap()(bitmap.as_bitmap_ptr()) }
    FramebufferStencilBitmap::new(bitmap)
  }

  /// Sets the font used for drawing.
  ///
  /// The font will remain active for drawing as long as the ActiveFont is not dropped, or another
  /// call to set_font() is made.
  pub fn set_font<'a>(&mut self, font: &'a Font) -> ActiveFont<'a> {
    unsafe { CApiState::get().cgraphics.setFont.unwrap()(font.as_ptr() as *mut CLCDFont) }
    ActiveFont::new(font)
  }

  /// Sets the current clip rect, using world coordinates—​that is, the given rectangle will be
  /// translated by the current drawing offset.
  ///
  /// The clip rect is cleared at the beginning of each frame.
  pub fn set_clip_rect(&mut self, rect: euclid::default::Rect<i32>) {
    unsafe {
      CApiState::get().cgraphics.setClipRect.unwrap()(
        rect.origin.x,
        rect.origin.y,
        rect.size.width,
        rect.size.height,
      )
    }
  }
  /// Sets the current clip rect in screen coordinates.
  ///
  /// The clip rect is cleared at the beginning of each frame.
  pub fn set_screen_clip_rect(&mut self, rect: euclid::default::Rect<i32>) {
    unsafe {
      CApiState::get().cgraphics.setScreenClipRect.unwrap()(
        rect.origin.x,
        rect.origin.y,
        rect.size.width,
        rect.size.height,
      )
    }
  }

  // TODO: all the graphics->video functions

  /// Sets the mode used for drawing bitmaps. Note that text drawing uses bitmaps, so this
  /// affects how fonts are displayed as well.
  pub fn set_draw_mode(&mut self, mode: BitmapDrawMode) {
    unsafe { CApiState::get().cgraphics.setDrawMode.unwrap()(mode) }
  }

  /// Draws the bitmap to the screen.
  ///
  /// The bitmap's upper-left corner is positioned at location (`x`, `y`), and the contents have
  /// the `flip` orientation applied.
  pub fn draw_bitmap(&mut self, bitmap: &BitmapRef, x: i32, y: i32, flip: BitmapFlip) {
    unsafe { CApiState::get().cgraphics.drawBitmap.unwrap()(bitmap.as_bitmap_ptr(), x, y, flip) }
  }

  /// Draws the bitmap to the screen, scaled by `xscale` and `yscale`.
  ///
  /// /// The bitmap's upper-left corner is positioned at location (`x`, `y`). Note that flip is not
  /// available when drawing scaled bitmaps but negative scale values will achieve the same effect.
  pub fn draw_scaled_bitmap(
    &mut self,
    bitmap: &BitmapRef,
    x: i32,
    y: i32,
    xscale: f32,
    yscale: f32,
  ) {
    unsafe {
      CApiState::get().cgraphics.drawScaledBitmap.unwrap()(
        bitmap.as_bitmap_ptr(),
        x,
        y,
        xscale,
        yscale,
      )
    }
  }

  /// Draws the bitmap to the screen, scaled by `xscale` and `yscale` then rotated by `degrees` with
  /// its center as given by proportions `centerx` and `centery` at (`x`, `y`); that is: if
  /// `centerx` and `centery` are both 0.5 the center of the image is at (`x`, `y`), if `centerx`
  /// and `centery` are both 0 the top left corner of the image (before rotation) is at (`x`, `y`),
  /// etc.
  pub fn draw_rotated_bitmap(
    &mut self,
    bitmap: &BitmapRef,
    x: i32,
    y: i32,
    degrees: f32,
    centerx: f32,
    centery: f32,
    xscale: f32,
    yscale: f32,
  ) {
    unsafe {
      CApiState::get().cgraphics.drawRotatedBitmap.unwrap()(
        bitmap.as_bitmap_ptr(),
        x,
        y,
        degrees,
        centerx,
        centery,
        xscale,
        yscale,
      )
    }
  }

  /// Draws the bitmap to the screen with its upper-left corner at location (`x`, `y`) tiled inside
  /// a `width` by `height` rectangle.
  pub fn draw_tiled_bitmap(
    &mut self,
    bitmap: &BitmapRef,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    flip: BitmapFlip,
  ) {
    unsafe {
      CApiState::get().cgraphics.tileBitmap.unwrap()(
        bitmap.as_bitmap_ptr(),
        x,
        y,
        width,
        height,
        flip,
      )
    }
  }

  // TODO: getTableBitmap
  // TODO: loadBitmapTable
  // TODO: loadIntoBitmapTable
  // TODO: newBitmapTable

  pub fn draw_text(&mut self, text: &str, encoding: StringEncoding, x: i32, y: i32) {
    let null_term = text.to_null_terminated_utf8();
    let ptr = null_term.as_ptr() as *const c_void;
    let len = null_term.len() as u64;
    unsafe { CApiState::get().cgraphics.drawText.unwrap()(ptr, len, encoding, x, y) }; // TODO: Return the int from Playdate?
  }

  /// Draws the current FPS on the screen at the given (`x`, `y`) coordinates.
  pub fn draw_fps(&mut self, x: i32, y: i32) {
    // This function is part of Playdate CSystemApi, not CGraphicsApi, but it's a function that draws
    // something to the screen, so its behaviour is more clear when part of the Graphics type.
    unsafe { CApiState::get().csystem.drawFPS.unwrap()(x, y) }
  }

  /// Draws an ellipse inside the rectangle of width `line_width` (inset from the rectangle bounds).
  ///
  /// If `start_deg != end_deg`, this draws an arc between the given angles. Angles are given in
  /// degrees, clockwise from due north.
  pub fn draw_elipse<'a>(
    &mut self,
    rect: euclid::default::Rect<i32>,
    line_width: i32,
    start_deg: f32,
    end_deg: f32,
    color: Color<'a>,
  ) {
    unsafe {
      CApiState::get().cgraphics.drawEllipse.unwrap()(
        rect.origin.x,
        rect.origin.y,
        rect.size.width,
        rect.size.height,
        line_width,
        start_deg,
        end_deg,
        color.to_c_color(),
      )
    }
  }
  /// Fills an ellipse inside the rectangle.
  ///
  /// If `start_deg != end_deg`, this draws an arc between the given angles. Angles are given in
  /// degrees, clockwise from due north.
  pub fn fill_elipse<'a>(
    &mut self,
    rect: euclid::default::Rect<i32>,
    start_deg: f32,
    end_deg: f32,
    color: Color<'a>,
  ) {
    unsafe {
      CApiState::get().cgraphics.fillEllipse.unwrap()(
        rect.origin.x,
        rect.origin.y,
        rect.size.width,
        rect.size.height,
        start_deg,
        end_deg,
        color.to_c_color(),
      )
    }
  }
  /// Draws a line from `p1` to `p2` with a stroke width of `width`.
  pub fn draw_line<'a>(
    &mut self,
    p1: euclid::default::Point2D<i32>,
    p2: euclid::default::Point2D<i32>,
    line_width: i32,
    color: Color<'a>,
  ) {
    unsafe {
      CApiState::get().cgraphics.drawLine.unwrap()(
        p1.x,
        p1.y,
        p2.x,
        p2.y,
        line_width,
        color.to_c_color(),
      )
    }
  }
  /// Draws a `rect`.
  pub fn draw_rect<'a>(&mut self, r: euclid::default::Rect<i32>, color: Color<'a>) {
    unsafe {
      CApiState::get().cgraphics.drawRect.unwrap()(
        r.origin.x,
        r.origin.y,
        r.size.width,
        r.size.height,
        color.to_c_color(),
      )
    }
  }
  /// Draws a filled `rect`.
  pub fn fill_rect<'a>(&mut self, r: euclid::default::Rect<i32>, color: Color<'a>) {
    unsafe {
      CApiState::get().cgraphics.fillRect.unwrap()(
        r.origin.x,
        r.origin.y,
        r.size.width,
        r.size.height,
        color.to_c_color(),
      )
    }
  }
  /// Draws a filled triangle with points at `p1`, `p2`, and `p3`.
  pub fn fill_triangle<'a>(
    &mut self,
    p1: euclid::default::Point2D<i32>,
    p2: euclid::default::Point2D<i32>,
    p3: euclid::default::Point2D<i32>,
    color: Color<'a>,
  ) {
    unsafe {
      CApiState::get().cgraphics.fillTriangle.unwrap()(
        p1.x,
        p1.y,
        p2.x,
        p2.y,
        p3.x,
        p3.y,
        color.to_c_color(),
      )
    }
  }
  /// Fills the polygon with vertices at the given coordinates (an array of points) using the given color and fill, or winding, rule.
  ///
  /// See <https://en.wikipedia.org/wiki/Nonzero-rule> for an explanation of the winding rule.
  pub fn fill_polygon<'a>(
    &mut self,
    points: &[euclid::default::Point2D<i32>],
    color: Color<'a>,
    fill_rule: PolygonFillRule,
  ) {
    // Point2D is a #[repr(C)] struct of x, y. It's alignment will be the same as i32, so an
    // array of Point2D can be treated as an array of i32 with x and y alternating.
    unsafe {
      CApiState::get().cgraphics.fillPolygon.unwrap()(
        points.len() as i32,
        points.as_ptr() as *mut i32,
        color.to_c_color(),
        fill_rule,
      )
    }
  }
}
