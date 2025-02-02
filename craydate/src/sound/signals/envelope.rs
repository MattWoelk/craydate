use alloc::rc::Rc;
use core::ptr::NonNull;

use super::synth_signal::{SynthSignal, SynthSignalSubclass};
use crate::capi_state::CApiState;
use crate::{ctypes::*, TimeTicks};

/// Holds (refcounted) ownership of the C Api object inside the SynthSignal.
struct EnvelopeSubclass {
  ptr: NonNull<CSynthEnvelope>,
}
impl Drop for EnvelopeSubclass {
  fn drop(&mut self) {
    unsafe { Envelope::fns().freeEnvelope.unwrap()(self.ptr.as_ptr()) }
  }
}
impl SynthSignalSubclass for EnvelopeSubclass {}

/// An Envelope is used to modulate sounds in a `Synth`.
///
/// BUG: Some functions are missing here as they are missing from the C API, as described here:
/// <https://devforum.play.date/t/c-apis-envelope-is-missing-some-functions-from-the-lua-apis/4925>
/// - setScale
/// - setOffset
/// - trigger
/// - setGlobal
pub struct Envelope {
  signal: SynthSignal,
  subclass: Rc<EnvelopeSubclass>,
}
impl Envelope {
  fn from_ptr(ptr: *mut CSynthEnvelope) -> Self {
    let subclass = Rc::new(EnvelopeSubclass {
      ptr: NonNull::new(ptr).unwrap(),
    });
    let signal = SynthSignal::new(ptr as *mut CSynthSignalValue, subclass.clone());
    Envelope { signal, subclass }
  }

  /// Constructs a new `Envelope`.
  ///
  ///  See `set_attack()`, `set_decay()`, `set_sustain()`, and `set_release()` for more details on
  ///  the parameters.
  pub fn new(attack: TimeTicks, decay: TimeTicks, sustain: f32, release: TimeTicks) -> Self {
    let ptr = unsafe {
      Self::fns().newEnvelope.unwrap()(
        attack.to_seconds(),
        decay.to_seconds(),
        sustain,
        release.to_seconds(),
      )
    };
    Self::from_ptr(ptr)
  }

  /// Sets the envelope attack time to `attack`.
  pub fn set_attack(&mut self, attack: TimeTicks) {
    unsafe { Self::fns().setAttack.unwrap()(self.cptr_mut(), attack.to_seconds()) }
  }
  /// Sets the envelope decay time to `decay`.
  pub fn set_decay(&mut self, decay: TimeTicks) {
    unsafe { Self::fns().setDecay.unwrap()(self.cptr_mut(), decay.to_seconds()) }
  }
  /// Sets the envelope sustain level to `sustain`, as a proportion of the maximum.
  ///
  /// For example, if the sustain level is 0.5, the signal value rises to its full value over the
  /// attack phase of the envelope, then drops to half its maximum over the decay phase, and remains
  /// there while the envelope is active.
  pub fn set_sustain_level(&mut self, sustain: f32) {
    unsafe { Self::fns().setSustain.unwrap()(self.cptr_mut(), sustain) }
  }
  /// Sets the envelope release time to `release`.
  pub fn set_release(&mut self, release: TimeTicks) {
    unsafe { Self::fns().setRelease.unwrap()(self.cptr_mut(), release.to_seconds()) }
  }

  /// Sets whether to use legato phrasing for the envelope.
  ///
  /// If the legato flag is set, when the envelope is re-triggered before it’s released, it remains
  /// in the sustain phase instead of jumping back to the attack phase.
  pub fn set_legato(&mut self, legato: bool) {
    unsafe { Self::fns().setLegato.unwrap()(self.cptr_mut(), legato as i32) }
  }

  /// Sets whether to start from 0 when playing a note.
  ///
  /// If retrigger is on, the envelope always starts from 0 when a note starts playing, instead of
  /// the current value if it’s active.
  pub fn set_retrigger(&mut self, retrigger: bool) {
    unsafe { Self::fns().setRetrigger.unwrap()(self.cptr_mut(), retrigger as i32) }
  }

  /// Return the current output value of the `Envelope`.
  pub fn get_value(&self) -> f32 {
    // getValue() takes a mutable pointer but it doesn't change any visible state.
    unsafe { Self::fns().getValue.unwrap()(self.cptr() as *mut _) }
  }

  pub(crate) fn cptr(&self) -> *const CSynthEnvelope {
    self.subclass.ptr.as_ptr()
  }
  pub(crate) fn cptr_mut(&mut self) -> *mut CSynthEnvelope {
    self.subclass.ptr.as_ptr()
  }
  pub(crate) fn fns() -> &'static craydate_sys::playdate_sound_envelope {
    unsafe { &*CApiState::get().csound.envelope }
  }
}

impl AsRef<SynthSignal> for Envelope {
  fn as_ref(&self) -> &SynthSignal {
    &self.signal
  }
}
impl AsMut<SynthSignal> for Envelope {
  fn as_mut(&mut self) -> &mut SynthSignal {
    &mut self.signal
  }
}
