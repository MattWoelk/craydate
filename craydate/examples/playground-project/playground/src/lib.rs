#![no_std]
#![deny(clippy::all)]
#![feature(never_type)]

use alloc::vec;
use alloc::vec::Vec;
use core::f32::consts::PI;

use craydate::*;
use euclid::{Point2D, Rect, Size2D, UnknownUnit};
use micromath::F32Ext;
use nalgebra::Vector2 as Vec2;

extern crate alloc;

#[derive(Default)]
struct ChainPoint {
  /// 0 is the most recent
  positions: Vec<Vec2<f32>>,
  length: f32,
  blur: bool,
}

impl ChainPoint {
  fn new(length: f32) -> ChainPoint {
    ChainPoint {
      positions: vec![Vec2::default(); 20], // TODO: make 20 into blur_frames
      length,
      blur: false,
      ..Default::default()
    }
  }

  fn blur(mut self, blur: bool) -> Self {
    self.blur = blur;
    self
  }
}

struct Weapon {
  chain: Vec<ChainPoint>,
  handle_length: f32, // TODO: use this
  stiffness: i32,
  blur_frames: usize,
}

#[craydate::main]
async fn main(mut api: craydate::Api) -> ! {
  let graphics = &mut api.graphics;

  let mut grey50 = Bitmap::new(8, 8, SolidColor::kColorBlack);
  for x in (0..8).step_by(2) {
    for y in 0..8 {
      let xwrite = x + y % 2;
      let ywrite = y;
      grey50.as_pixels_mut().set(xwrite, ywrite, PixelColor::WHITE)
    }
  }
  let _grey50 = Pattern::from_bitmap(&grey50, 0, 0);
  let mut grey50_colors = [PixelColor::BLACK; 8 * 8];
  for x in 0..8 {
    for y in 0..8 {
      let xodd = x % 2 != 0;
      let yodd = y % 2 != 0;
      if yodd == xodd {
        grey50_colors[y * 8 + x] = PixelColor::WHITE;
      }
    }
  }
  let grey50 = Pattern::new_unmasked(grey50_colors);
  graphics.clear(&grey50);

  //let mut bmp = Bitmap::new(100, 40, SolidColor::kColorWhite);
  //let mask = Bitmap::new(100, 40, SolidColor::kColorWhite);
  //bmp.set_mask_bitmap(&mask).expect("mask problems");

  //graphics.draw_bitmap(&bmp, 5, 9, BitmapFlip::kBitmapUnflipped);

  //let mut stencil = Bitmap::new(64, 64, SolidColor::kColorWhite);
  //for y in 0..64 as usize {
  //  let c = y % 4 != 0;
  //  for x in 0..64 as usize {
  //    stencil.as_pixels_mut().set(x, y, c.into());
  //  }
  //}

  let font = Font::from_file("Mini Sans 2X.pft");
  let _active = match &font {
    Ok(font) => {
      log(format!("Font height: {}", font.font_height()));

      let page = font.font_page('d');
      log("Got page");
      let _bitmap = page.glyph('d').unwrap().bitmap();

      Some(graphics.set_font(font))
    }
    Err(e) => {
      log(format!("ERROR: loading font {}", e));
      None
    }
  };

  {
    //let _stencil_holder = graphics.set_stencil(&stencil);
    graphics.draw_text("Bloop", 30, 20);
  }

  let mut copy = graphics.working_frame_bitmap();

  for y in 0..240 {
    for x in 0..400 {
      let val = (x as f32 / 7.4).sin();
      let val = val + 0.5 * (x as f32 / 3.4).sin();
      let val = val + (y as f32 / 4.0).sin();
      let val = val + 0.5 * (y as f32 / 2.2).sin();
      copy.as_pixels_mut().set(
        x,
        y,
        if val > 0f32 {
          PixelColor::BLACK
        } else {
          PixelColor::WHITE
        },
      );
    }
  }
  graphics.draw_bitmap(&copy, 0, 30, BitmapFlip::kBitmapUnflipped);
  graphics.push_context_bitmap(copy);
  graphics.pop_context();

  let mut i32callbacks = Callbacks::<i32>::new();

  log(format!(
    "Entering main loop at time {}",
    api.system.current_time()
  ));

  let events = api.system.system_event_watcher();

  let mut current_weapon = 0;
  let mut weapons = vec![
    Weapon {
      chain: vec![
        ChainPoint::new(75.),
        ChainPoint::new(30.),
        ChainPoint::new(30.).blur(true),
        ChainPoint::new(75.),
      ],
      handle_length: 75.,
      stiffness: 10,
      blur_frames: 1,
    },
    Weapon {
      chain: vec![
        ChainPoint::new(75.),
        ChainPoint::new(15.),
        ChainPoint::new(15.),
        ChainPoint::new(15.),
        ChainPoint::new(15.),
        ChainPoint::new(15.),
        ChainPoint::new(15.),
        ChainPoint::new(15.),
        ChainPoint::new(15.).blur(true),
        ChainPoint::new(15.),
      ],
      handle_length: 75.,
      stiffness: 20,
      blur_frames: 4,
    },
    Weapon {
      chain: vec![
        ChainPoint::new(30.),
        ChainPoint::new(125.).blur(true),
        ChainPoint::new(125.), // TODO: why is this required ??? Does the length on the last one not matter? Hmm.....
      ],
      handle_length: 75.,
      stiffness: 10,
      blur_frames: 2,
    },
  ];

  let origin = Point2D::new(100, 120);

  let mut shield_offset: f32 = 0.;
  let mut shield_position = p_to_v(&origin);
  shield_position.x += 90.;

  // This is the main game
  loop {
    let (inputs, frame_number) = match events.next().await {
      SystemEvent::NextFrame {
        inputs,
        frame_number,
      } => (inputs, frame_number),
      SystemEvent::WillLock => {
        log("locked");
        continue;
      }
      SystemEvent::DidUnlock => {
        log("unlocked");
        continue;
      }
      SystemEvent::Callback => {
        i32callbacks.run(1);
        continue;
      }
      _ => continue,
    };

    for (button, event) in inputs.buttons().all_events() {
      match event {
        craydate::ButtonEvent::Push => {
          log(format!("{:?} pushed on frame {}", button, frame_number));
          current_weapon = (current_weapon + 1) % weapons.len();
        }
        craydate::ButtonEvent::Release => {
          log(format!("{:?} released on frame {}", button, frame_number));
        }
      }
    }

    // Draw Background:
    let graphics = &mut api.graphics;
    let grey50 = Pattern::new_unmasked(grey50_colors);
    graphics.clear(&grey50);

    let mut chain_start = Vec2::default();

    match inputs.crank() {
      Crank::Undocked {
        angle,
        change: angle_delta,
      } => {
        let angle = (angle - 90.) * PI / 180.; // TODO: this is hackery to flip the y axis. :'( It should probably be '+'

        shield_offset = (shield_offset - angle_delta).clamp(-190., 0.);

        let length = 75f32;
        let direction: Point2D<i32, UnknownUnit> =
          Point2D::new((angle.cos() * length) as i32, (angle.sin() * length) as i32);

        let destination = Point2D::new(origin.x + direction.x, origin.y + direction.y);
        chain_start = Vec2::new(destination.x as f32, destination.y as f32);

        api.graphics.draw_line(
          origin,
          destination,
          3,
          Color::Solid(SolidColor::kColorBlack),
        )
      }
      _ => {}
    }

    let blur_frames = weapons[current_weapon].blur_frames;
    let stiffness = weapons[current_weapon].stiffness;
    let mut chain = &mut weapons[current_weapon].chain;

    // Solve Chain
    move_chain(&mut chain, chain_start, blur_frames);
    for _ in 0..stiffness {
      constrain_chain_lengths(&mut chain);
    }

    // Draw Chain
    chain.windows(2).for_each(|links| {
      api.graphics.draw_line(
        v_to_p(&links[0].positions[0]),
        v_to_p(&links[1].positions[0]),
        3,
        Color::Solid(if links[0].blur {
          SolidColor::kColorWhite
        } else {
          SolidColor::kColorBlack
        }),
      );
    });

    // Draw motion blur
    for p in 0..blur_frames {
      for l in 0..chain.len() - 1 {
        if chain[l].blur {
          api.graphics.fill_polygon(
            &[
              v_to_p(&chain[l].positions[p]),
              v_to_p(&chain[l].positions[p + 1]),
              v_to_p(&chain[l + 1].positions[p + 1]),
              v_to_p(&chain[l + 1].positions[p]),
            ],
            Color::Solid(SolidColor::kColorWhite),
            PolygonFillRule::kPolygonFillNonZero,
          );
        }
      }
    }

    // Draw Shield
    let shield_width = 40;
    let shield_height = 80;
    api.graphics.draw_rect(
      Rect {
        origin: v_to_p(
          &(shield_position
            - Vec2::new(
              shield_width as f32 / 2.,
              shield_height as f32 / 2. + shield_offset / 3.,
            )),
        ),
        size: Size2D::new(shield_width, shield_height),
      },
      Color::Solid(SolidColor::kColorWhite),
    );

    // Draw fps
    api.graphics.draw_fps(400 - 15, 0);
  }
}

fn v_to_p(v: &Vec2<f32>) -> Point2D<i32, UnknownUnit> {
  Point2D::new(v.x as i32, v.y as i32)
}

fn p_to_v(p: &Point2D<i32, UnknownUnit>) -> Vec2<f32> {
  Vec2::new(p.x as f32, p.y as f32)
}

fn move_chain(chain: &mut [ChainPoint], chain_start: Vec2<f32>, blur_frames: usize) {
  let grav = 3.9;
  let drag = 1.0;

  chain.iter_mut().enumerate().for_each(|(i, link)| {
    let delta = (link.positions[0] - link.positions[1]) * drag;

    // backup the previous positions
    for i in (0..blur_frames).rev() {
      link.positions[i + 1] = link.positions[i];
    }

    // If 1st link, set it to chain_start
    if i == 0 {
      link.positions[0] = chain_start;
    } else {
      link.positions[0] += delta;
      link.positions[0].y += grav;
    }
  });
}

fn constrain_chain_lengths(chain: &mut [ChainPoint]) {
  if chain.len() < 2 {
    return;
  }

  // first link, where its base does not move
  let a = chain[0].positions[0];
  let b = chain[1].positions[0];
  let delta = b - a;
  let distance = (delta.x * delta.x + delta.y * delta.y).sqrt();
  let fraction = (30. - distance) / distance; // TODO: this needs to use arm_length
  if fraction < 0.0 {
    let delta = delta * fraction;
    chain[1].positions[0] = b + delta;
  }

  // the rest of the chain
  for i in 1..(chain.len() - 1) {
    let a = chain[i].positions[0];
    let b = chain[i + 1].positions[0];
    let delta = b - a;
    let distance = (delta.x * delta.x + delta.y * delta.y).sqrt();
    let link_length = chain[i + 1].length;
    let fraction = ((link_length - distance) / distance) / 2.;
    if fraction < 0.0 {
      let delta = delta * fraction;
      chain[i].positions[0] = a - delta;
      chain[i + 1].positions[0] = b + delta;
    }
  }
}
