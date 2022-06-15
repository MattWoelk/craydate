#![no_std]
#![deny(clippy::all)]
#![feature(never_type)]

use core::f32::consts::PI;

use craydate::*;
use euclid::{Point2D, Rect, Size2D, UnknownUnit};
use micromath::F32Ext;
use nalgebra::Vector2 as Vec2;

extern crate alloc;

#[derive(Default)]
struct ChainPoint {
  position: Vec2<f32>,
  prev: Vec2<f32>,
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

  let mut chain = [
    ChainPoint::default(),
    ChainPoint::default(),
    ChainPoint::default(),
    ChainPoint::default(),
  ];

  let origin = Point2D::new(300, 120);

  let mut shield_offset: f32 = 0.;
  let mut shield_position = p_to_v(&origin);
  shield_position.x -= 90.;

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

        shield_offset = (shield_offset + angle_delta).clamp(-190., 0.);

        let length = 75f32;
        let direction: Point2D<i32, UnknownUnit> =
          Point2D::new((angle.cos() * length) as i32, (angle.sin() * length) as i32);

        let destination = Point2D::new(origin.x + direction.x, origin.y + direction.y);
        chain_start = Vec2::new(destination.x as f32, destination.y as f32);

        api.graphics.draw_line(
          origin,
          destination,
          3,
          Color::Solid(SolidColor::kColorWhite),
        )
      }
      _ => {}
    }

    // Solve Chain
    move_chain(&mut chain);
    for _ in 0..10 {
      constrain_chain_lengths(&chain_start, &mut chain);
    }

    // Draw Chain
    api.graphics.draw_line(
      v_to_p(&chain_start),
      v_to_p(&chain[0].position),
      3,
      Color::Solid(SolidColor::kColorWhite),
    );
    chain.windows(2).for_each(|links| {
      api.graphics.draw_line(
        v_to_p(&links[0].position),
        v_to_p(&links[1].position),
        3,
        Color::Solid(SolidColor::kColorWhite),
      );
    });

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

fn move_chain(chain: &mut [ChainPoint]) {
  let grav = 3.9;
  let drag = 1.0;

  chain.iter_mut().for_each(|link| {
    let delta = (link.position - link.prev) * drag;
    link.prev = link.position;
    link.position += delta;
    link.position.y += grav
  });
}

fn constrain_chain_lengths(chain_start: &Vec2<f32>, chain: &mut [ChainPoint]) {
  if chain.len() < 2 {
    return;
  }

  // first link, relative to chain_start
  let b = chain[0].position;
  let delta = b - chain_start;
  let distance = (delta.x * delta.x + delta.y * delta.y).sqrt();
  let fraction = (30. - distance) / distance;
  if fraction < 0.0 {
    let delta = delta * fraction;
    chain[0].position = b + delta;
  }

  // the rest of the chain
  for i in 0..(chain.len() - 1) {
    let a = chain[i].position;
    let b = chain[i + 1].position;
    let delta = b - a;
    let distance = (delta.x * delta.x + delta.y * delta.y).sqrt();
    let fraction = ((30. - distance) / distance) / 2.;
    if fraction < 0.0 {
      let delta = delta * fraction;
      chain[i].position = a - delta;
      chain[i + 1].position = b + delta;
    }
  }
}
