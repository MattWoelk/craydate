use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use core::ffi::c_void;
use core::mem::ManuallyDrop;
use core::ptr::NonNull;

use super::super::audio_sample::AudioSample;
use super::super::midi::track_note::TrackNote;
use super::super::signals::synth_signal::SynthSignal;
use super::super::volume::Volume;
use super::sound_source::SoundSource;
use crate::capi_state::CApiState;
use crate::ctypes::*;
use crate::ctypes_enums::SoundWaveform;
use crate::error::Error;
use crate::time::{TimeDelta, TimeSpan, TimeTicks};

/// A collection of `Synth` objects make up an `Instrument` used to play a MIDI `Sequence`.
///
/// A `Synth` is also a `SoundSource` and thus can be played to a `SoundChannel` directly.
///
/// A `Synth` can generate sound from a fixed function, which is a `SoundWaveform`. Or it can play
/// sound from an `AudioSample`, or the user can provide their own function as a `SynthGenerator`.
#[derive(Debug)]
pub struct Synth {
  source: ManuallyDrop<SoundSource>,
  ptr: NonNull<CSynth>,
  frequency_modulator: Option<SynthSignal>,
  amplitude_modulator: Option<SynthSignal>,
  parameter_modulators: BTreeMap<i32, SynthSignal>,
  sample: Option<AudioSample>, // Set if constructed from an AudioSample.
}
impl Synth {
  /// Creates a new Synth.
  fn new() -> Self {
    let ptr = unsafe { Self::fns().newSynth.unwrap()() };
    Synth {
      source: ManuallyDrop::new(SoundSource::from_ptr(ptr as *mut CSoundSource)),
      ptr: NonNull::new(ptr).unwrap(),
      frequency_modulator: None,
      amplitude_modulator: None,
      parameter_modulators: BTreeMap::new(),
      sample: None,
    }
  }

  /// Creates a new Synth that plays a waveform.
  pub fn new_with_waveform(waveform: SoundWaveform) -> Self {
    let mut synth = Self::new();
    unsafe { Self::fns().setWaveform.unwrap()(synth.cptr_mut(), waveform) };
    synth
  }

  /// Creates a new Synth that plays a sample.
  ///
  /// An optional sustain region defines a loop to play while the note is on. Sample data must be
  /// uncompressed PCM, not ADPCM.
  pub fn new_with_sample(sample: AudioSample, sustain_region: Option<TimeSpan>) -> Synth {
    let mut synth = Self::new();
    unsafe {
      // setSample() takes a mutable pointer but doesn't mutate any visible state.
      Self::fns().setSample.unwrap()(
        synth.cptr_mut(),
        sample.cptr() as *mut _,
        sustain_region.as_ref().map_or(0, |r| r.start.to_sample_frames()),
        sustain_region.as_ref().map_or(0, |r| r.end.to_sample_frames()),
      )
    };
    synth.sample = Some(sample);
    synth
  }

  /// Creates a new Synth that plays from a `SynthGenerator`.
  ///
  /// BUG: THIS DOES NOT WORK!! See
  /// <https://devforum.play.date/t/c-api-playdate-sound-synth-setgenerator-has-incorrect-api/4482>
  /// as this is due to a Playdate bug.
  ///
  /// The `SynthGenerator` is a set of functions that are called in order to fill the sample buffers
  /// with data and react to events on the Synth object.
  pub fn new_with_generator(generator: SynthGenerator) -> Self {
    let mut synth = Self::new();
    unsafe {
      Self::fns().setGenerator.unwrap()(
        synth.cptr_mut(),
        // The Playdate C Api has incorrect types so we need to do some wild casting here:
        // https://devforum.play.date/t/c-api-playdate-sound-synth-setgenerator-has-incorrect-api/4482
        c_render_func as *mut Option<CRenderFunc>,
        c_note_on_func as *mut Option<CNoteOnFunc>,
        c_release_func as *mut Option<CReleaseFunc>,
        c_set_parameter_func as *mut Option<CSetParameterFunc>,
        c_dealloc_func as *mut Option<CDeallocFunc>,
        // The generator vtable includes a dealloc function which will be responsible for dropping
        // this `Box<SynthGenerator>`.
        Box::into_raw(Box::new(generator)) as *mut c_void,
      )
    };
    synth
  }

  /// Sets the attack time for the sound envelope.
  pub fn set_attack_time(&mut self, attack_time: TimeDelta) {
    unsafe { Self::fns().setAttackTime.unwrap()(self.cptr_mut(), attack_time.to_seconds()) }
  }
  /// Sets the decay time for the sound envelope.
  pub fn set_decay_time(&mut self, decay_time: TimeDelta) {
    unsafe { Self::fns().setDecayTime.unwrap()(self.cptr_mut(), decay_time.to_seconds()) }
  }
  /// Sets the sustain level, from 0 to 1, for the sound envelope.
  pub fn set_sustain_level(&mut self, level: f32) {
    unsafe { Self::fns().setSustainLevel.unwrap()(self.cptr_mut(), level) }
  }
  /// Sets the release time for the sound envelope.
  pub fn set_release_time(&mut self, release_time: TimeDelta) {
    unsafe { Self::fns().setReleaseTime.unwrap()(self.cptr_mut(), release_time.to_seconds()) }
  }
  /// Transposes the synth’s output by the given number of half steps.
  ///
  /// For example, if the transpose is set to 2 and a C note is played, the synth will output a D
  /// instead.
  pub fn set_transpose(&mut self, half_steps: f32) {
    unsafe { Self::fns().setTranspose.unwrap()(self.cptr_mut(), half_steps) }
  }

  /// Sets a signal to modulate the `Synth`’s frequency.
  ///
  /// The signal is scaled so that a value of 1 doubles the synth pitch (i.e. an octave up) and -1
  /// halves it (an octave down).
  ///
  /// The signal is cloned, which is a shallow copy, so the caller can retain the ability to mutate
  /// the signal.
  pub fn set_frequency_modulator<T: AsRef<SynthSignal>>(&mut self, signal: Option<&T>) {
    let modulator_ptr = signal.map_or_else(core::ptr::null_mut, |signal|
      // setFrequencyModulator() takes a mutable pointer to the modulator but there is no visible
      // state on the modulator.
      signal.as_ref().cptr() as *mut _);
    unsafe { Self::fns().setFrequencyModulator.unwrap()(self.cptr_mut(), modulator_ptr) }
    self.frequency_modulator = signal.map(|signal| signal.as_ref().clone());
  }
  /// Gets the current signal modulating the `Synth`'s frequency.
  pub fn frequency_modulator(&mut self) -> Option<&SynthSignal> {
    self.frequency_modulator.as_ref()
  }

  /// Sets a signal to modulate the `Synth`’s output amplitude.
  ///
  /// The signal is cloned, which is a shallow copy, so the caller can retain the ability to mutate
  /// the signal.
  pub fn set_amplitude_modulator<T: AsRef<SynthSignal>>(&mut self, signal: Option<&T>) {
    let modulator_ptr = signal.map_or_else(core::ptr::null_mut, |signal|
      // setAmplitudeModulator() takes a mutable pointer to the modulator but there is no visible
      // state on the modulator.
      signal.as_ref().cptr() as *mut _);
    unsafe { Self::fns().setAmplitudeModulator.unwrap()(self.cptr_mut(), modulator_ptr) }
    self.amplitude_modulator = signal.map(|signal| signal.as_ref().clone());
  }
  /// Gets the current signal modulating the `Synth`’s output amplitude.
  pub fn amplitude_modulator(&mut self) -> Option<&SynthSignal> {
    self.amplitude_modulator.as_ref()
  }

  /// Sets a signal to modulate the parameter at index `i`.
  ///
  /// The signal is cloned, which is a shallow copy, so the caller can retain the ability to mutate
  /// the signal.
  pub fn set_parameter_modulator<T: AsRef<SynthSignal>>(&mut self, i: i32, signal: Option<&T>) {
    let modulator_ptr = signal.map_or_else(core::ptr::null_mut, |signal|
      // setParameterModulator() takes a mutable pointer to the modulator but there is no visible
      // state on the modulator.
      signal.as_ref().cptr() as *mut _);
    unsafe { Self::fns().setParameterModulator.unwrap()(self.cptr_mut(), i, modulator_ptr) }
    match signal.map(|signal| signal.as_ref().clone()) {
      Some(signal) => self.parameter_modulators.insert(i, signal),
      None => self.parameter_modulators.remove(&i),
    };
  }
  /// Gets the current signal modulating the parameter at index `i`.
  pub fn parameter_modulator(&mut self, i: i32) -> Option<&SynthSignal> {
    self.parameter_modulators.get(&i)
  }

  /// Returns the number of parameters advertised by the Synth.
  pub fn parameter_count(&self) -> i32 {
    // getParameterCount() takes a mutable pointer but doesn't change any visible state.
    unsafe { Self::fns().getParameterCount.unwrap()(self.cptr() as *mut _) }
  }
  /// Set the Synth's `i`th parameter to `value`.
  ///
  /// `i` is 0-based, so the first parameter is `0`, the second is `1`, etc. Returns
  /// `Error::NotFoundError` is the parameter `i` is not valid.
  pub fn set_parameter(&mut self, i: i32, value: f32) -> Result<(), Error> {
    let r = unsafe { Self::fns().setParameter.unwrap()(self.cptr_mut(), i, value) };
    match r {
      0 => Err(Error::NotFoundError),
      _ => Ok(()),
    }
  }

  /// Plays a note on the Synth, using the `frequency`.
  ///
  /// If `length` is `None`, the note will continue playing until a subsequent `stop()` call. If
  /// `when` is None, the note is played immediately, otherwise the note is scheduled for the given
  /// absolute time. Use `Sound::current_sound_time()` to get the current time.
  pub fn play_frequency_note(
    &mut self,
    frequency: f32,
    volume: Volume,
    length: Option<TimeDelta>,
    when: Option<TimeTicks>,
  ) {
    unsafe {
      Self::fns().playNote.unwrap()(
        self.cptr_mut(),
        frequency,
        volume.into(),
        length.map_or(-1.0, |l| l.to_seconds()),
        when.map_or(0, |w| w.to_sample_frames()),
      )
    }
  }

  /// Plays a MIDI note on the Synth, where 'C4' is `60.0` for the `note`.
  ///
  /// If `length` is `None`, the note will continue playing until a subsequent `stop()` call. If
  /// `when` is None, the note is played immediately, otherwise the note is scheduled for the given
  /// absolute time. Use `Sound::current_sound_time()` to get the current time.
  pub fn play_midi_note(
    &mut self,
    note: TrackNote,
    length: Option<TimeDelta>,
    when: Option<TimeTicks>,
  ) {
    unsafe {
      Self::fns().playMIDINote.unwrap()(
        self.cptr_mut(),
        note.midi_note as f32,
        note.velocity.into(),
        length.map_or(-1.0, |l| l.to_seconds()),
        when.map_or(0, |w| w.to_sample_frames()),
      )
    }
  }

  /// Stops the currently play8iung note.
  ///
  /// If `when` is `None`, the note is stopped immediately. Otherwise it is scheduled to be stopped
  /// at the given absolute time. Use `Sound::current_sound_time()` to get the current time.
  pub fn stop(&mut self, when: Option<TimeTicks>) {
    unsafe {
      Self::fns().noteOff.unwrap()(self.cptr_mut(), when.map_or(0, |w| w.to_sample_frames()))
    }
  }

  pub(crate) fn cptr(&self) -> *const CSynth {
    self.ptr.as_ptr()
  }
  pub(crate) fn cptr_mut(&mut self) -> *mut CSynth {
    self.ptr.as_ptr()
  }
  fn fns() -> &'static craydate_sys::playdate_sound_synth {
    unsafe { &*CApiState::get().csound.synth }
  }
}

impl Drop for Synth {
  fn drop(&mut self) {
    // Ensure the SoundSource has a chance to clean up before it is freed.
    unsafe { ManuallyDrop::drop(&mut self.source) };
    // The AudioSample will be freed after the `Synth` which references it.
    unsafe { Self::fns().freeSynth.unwrap()(self.cptr_mut()) };
  }
}

impl AsRef<SoundSource> for Synth {
  fn as_ref(&self) -> &SoundSource {
    &self.source
  }
}
impl AsMut<SoundSource> for Synth {
  fn as_mut(&mut self) -> &mut SoundSource {
    &mut self.source
  }
}

/// Parameters for the SynthGeneraterRenderFunc.
#[allow(dead_code)]
pub struct SynthRender<'a> {
  /// The left sample buffer in Q8.24 format.
  left: &'a mut [i32],
  /// The right sample buffer in Q8.24 format.
  right: &'a mut [i32],
  /// TODO: What is this?
  rate: u32,
  /// TODO: What is this?
  drate: i32,
  /// The left level value in Q4.28 format, used to scale the samples to follow the synth’s envelope
  /// and/or amplitude modulator levels.
  l: i32,
  /// The left slope value that should be added to `l` every frame.
  dl: i32,
  /// The right level value in Q4.28 format, used to scale the samples to follow the synth’s
  /// envelope and/or amplitude modulator levels.
  r: i32,
  /// The right slope value that should be added to `r` every frame.
  dr: i32,
}

/// A virtual function pointer table (vtable) that specifies the behaviour of a `SynthGenerator`.
///
/// The functions are only meant to be called as part of a SynthGenerator, and calling them in any
/// other context will cause undefined behaviour.
pub struct SynthGeneratorVTable {
  /// The data provider callback for a generator. The generator should add its samples to the data
  /// already in the `left` and `right` buffers in the `SynthRender`.
  ///
  /// The `render_func` should write data into the `SynthRender::left` and `SynthRender::right`
  /// buffers and return `true` if it has done so. Or it can return `false` to indicate that the
  /// generator is silent for the sound frame.
  pub render_func: fn(userdata: *const (), SynthRender<'_>) -> bool,
  /// TODO: What is this?
  pub note_on_func: fn(userdata: *const (), note: f32, volume: f32, length: Option<TimeTicks>),
  /// TODO: What is this?
  pub release_func: fn(userdata: *const (), ended: bool),
  /// TODO: Is this called in response to set_parameter()? What parameters go here verses elsewhere?
  /// How does get_parameters() know what to return? What is the return value? Is `bool` the right
  /// output value, or should be it `i32` like the C function?
  pub set_parameter_func: fn(userdata: *const (), parameter: u8, value: f32) -> bool,
}

/// The implementation of a generator for a `Synth`.
pub struct SynthGenerator {
  data: *const (),
  vtable: &'static SynthGeneratorVTable,
}
impl SynthGenerator {
  /// Construct a `SynthGenerator` that generates the sample data for a `Synth`.
  ///
  /// The `data` will be stored on the heap, and a pointer to it will be passed to all the methods
  /// in the `vtable` as the first parameter. The `vtable` defines the behaviour of the generator.
  pub fn new<T: Send + Sync>(data: T, vtable: &'static SynthGeneratorVTable) -> Self {
    SynthGenerator {
      data: Box::into_raw(Box::new(data)) as *const (),
      vtable,
    }
  }
}
impl Drop for SynthGenerator {
  fn drop(&mut self) {
    drop(unsafe { Box::from_raw(self.data as *mut ()) });
  }
}
impl core::fmt::Debug for SynthGenerator {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    // The vtable field is not representable.
    f.debug_struct("SynthGenerator").field("data", &self.data).finish()
  }
}

type CRenderFunc =
  unsafe extern "C" fn(*mut c_void, *mut i32, *mut i32, i32, u32, i32, i32, i32, i32, i32) -> i32;
unsafe extern "C" fn c_render_func(
  generator: *mut c_void,
  left: *mut i32,
  right: *mut i32,
  nsamples: i32,
  rate: u32,
  drate: i32,
  l: i32,
  dl: i32,
  r: i32,
  dr: i32,
) -> i32 {
  let generator = generator as *const SynthGenerator;
  let func = (*generator).vtable.render_func;
  let userdata = (*generator).data;
  func(
    userdata,
    SynthRender {
      left: alloc::slice::from_raw_parts_mut(left, nsamples as usize),
      right: alloc::slice::from_raw_parts_mut(right, nsamples as usize),
      rate,
      drate,
      l,
      dl,
      r,
      dr,
    },
  ) as i32
}
type CNoteOnFunc = unsafe extern "C" fn(*mut c_void, f32, f32, f32);
unsafe extern "C" fn c_note_on_func(generator: *mut c_void, note: f32, volume: f32, length: f32) {
  let generator = generator as *const SynthGenerator;
  let func = (*generator).vtable.note_on_func;
  let userdata = (*generator).data;
  // The length is -1 if indefinite, per
  // https://sdk.play.date/1.9.3/Inside%20Playdate%20with%20C.html#f-sound.synth.setGenerator.
  let length = if length == -1.0 {
    None
  } else {
    Some(TimeTicks::from_seconds_lossy(length))
  };
  func(userdata, note, volume, length)
}
type CReleaseFunc = unsafe extern "C" fn(*mut c_void, i32);
unsafe extern "C" fn c_release_func(generator: *mut c_void, ended: i32) {
  let generator = generator as *const SynthGenerator;
  let func = (*generator).vtable.release_func;
  let userdata = (*generator).data;
  func(userdata, ended != 0)
}
type CSetParameterFunc = unsafe extern "C" fn(*mut c_void, u8, f32) -> i32;
unsafe extern "C" fn c_set_parameter_func(
  generator: *mut c_void,
  parameter: u8,
  value: f32,
) -> i32 {
  let generator = generator as *const SynthGenerator;
  let func = (*generator).vtable.set_parameter_func;
  let userdata = (*generator).data;
  func(userdata, parameter, value) as i32
}
type CDeallocFunc = unsafe extern "C" fn(*mut c_void);
unsafe extern "C" fn c_dealloc_func(generator: *mut c_void) {
  // The generator `data` is dealloced by `SynthGenerator::drop()`.
  drop(Box::from_raw(generator as *mut SynthGenerator))
}
