use core::ptr::NonNull;

use crate::bitmap::SharedBitmapRef;
use crate::capi_state::CApiState;
use crate::ctypes::*;
use crate::null_terminated::ToNullTerminatedString;

/// Font which can be used to draw text when made active with `Graphics::set_font()`.
#[derive(Debug)]
pub struct Font {
  font_ptr: NonNull<CLCDFont>,
}
impl Font {
  pub(crate) fn from_ptr(font_ptr: *mut CLCDFont) -> Self {
    Font {
      font_ptr: unsafe { NonNull::new_unchecked(font_ptr) },
    }
  }

  /// Measure the `text` string as drawn with the font.
  ///
  /// The `tracking` value is the number of pixels of whitespace between each character drawn in a
  /// string.
  pub fn measure_text_width(&self, text: &str, tracking: i32) -> i32 {
    let utf = text.to_null_terminated_utf8();
    unsafe {
      CApiState::get().cgraphics.getTextWidth.unwrap()(
        self.font_ptr.as_ptr(),
        utf.as_ptr() as *const core::ffi::c_void,
        utf.len() as u64 - 1, // Don't count the null.
        StringEncoding::kUTF8Encoding,
        tracking,
      )
    }
  }

  /// The height of the font.
  pub fn font_height(&self) -> u8 {
    unsafe { CApiState::get().cgraphics.getFontHeight.unwrap()(self.font_ptr.as_ptr()) }
  }

  /// Returns the FontPage for the character `c`.
  ///
  /// Each FontPage contains information for 256 characters. All chars with the same high 24 bits
  /// share a page; specifically, if `(c1 & ~0xff) == (c2 & ~0xff)`, then c1 and c2 belong to the
  /// same page. The FontPage can be used to query information about all characters in the page.
  pub fn font_page(&self, c: char) -> FontPage {
    let page_ptr =
      unsafe { CApiState::get().cgraphics.getFontPage.unwrap()(self.font_ptr.as_ptr(), c as u32) };
    FontPage {
      page_ptr: unsafe { NonNull::new_unchecked(page_ptr) },
      page_test: c as u32 & 0xffffff00,
    }
  }

  pub(crate) fn as_ptr(&self) -> *mut CLCDFont {
    self.font_ptr.as_ptr()
  }
}

/// Information about a set of 256 chars.
///
/// All chars with the same high 24 bits share a page; specifically, if `(c1 & ~0xff) == (c2 &
/// ~0xff)`, then c1 and c2 belong to the same page. The FontPage can be used to query information
/// about all characters in the page.
pub struct FontPage {
  page_ptr: NonNull<CLCDFontPage>,
  /// If a characters high 24 bits match this, then it's part of the page.
  page_test: u32,
}
impl FontPage {
  /// Whether the FontPage contains information for the character `c`.
  ///
  /// Each FontPage contains information for 256 characters. All chars with the same high 24 bits
  /// share a page; specifically, if `(c1 & ~0xff) == (c2 & ~0xff)`, then c1 and c2 belong to the
  /// same page.
  pub fn contains(&self, c: char) -> bool {
    c as u32 & 0xffffff00 == self.page_test
  }

  /// Returns the glyph for the character `c`.
  ///
  /// May return None if the character is not part of this FontPage. Each FontPage contains
  /// information for 256 characters. All chars with the same high 24 bits share a page;
  /// specifically, if `(c1 & ~0xff) == (c2 & ~0xff)`, then c1 and c2 belong to the same page.
  pub fn glyph(&self, c: char) -> Option<FontGlyph> {
    if !self.contains(c) {
      None
    } else {
      // UNCLEAR: getPageGlyph says the `bitmap_ptr` and `advance` are optional but passing null
      // for either one crashes.
      let mut bitmap_ptr: *mut CLCDBitmap = core::ptr::null_mut();
      let mut advance = 0;
      let glyph_ptr = unsafe {
        CApiState::get().cgraphics.getPageGlyph.unwrap()(
          self.page_ptr.as_ptr(),
          c as u32,
          &mut bitmap_ptr,
          &mut advance,
        )
      };
      Some(FontGlyph {
        glyph_ptr: NonNull::new(glyph_ptr).unwrap(),
        advance,
        glyph_char: c,
        bitmap: SharedBitmapRef::<'static>::from_ptr(bitmap_ptr),
      })
    }
  }
}

/// Information about a specific character's font glyph.
pub struct FontGlyph {
  glyph_ptr: NonNull<CLCDFontGlyph>,
  advance: i32,
  glyph_char: char,
  // Fonts can not be unloaded/destroyed, so the bitmap has a static lifetime.
  bitmap: SharedBitmapRef<'static>,
}
impl FontGlyph {
  /// Returns the advance value for the glyph, which is the width that should be allocated for the
  /// glyph.
  pub fn advance(&self) -> i32 {
    self.advance
  }

  /// Returns the kerning adjustment between the glyph and `next_char` as specified by the font.
  ///
  /// The adjustment would be applied to the `advance()`.
  pub fn kerning(&self, next_char: char) -> i32 {
    unsafe {
      CApiState::get().cgraphics.getGlyphKerning.unwrap()(
        self.glyph_ptr.as_ptr(),
        self.glyph_char as u32,
        next_char as u32,
      )
    }
  }

  /// The bitmap representation of the font glyph.
  pub fn bitmap(&self) -> SharedBitmapRef<'static> {
    self.bitmap.clone()
  }
}