use egui::{Pos2, Rect};
use enigma_3d::camera::Camera;
use nalgebra::Vector3;

/// Build a right/up screen basis from `forward`. Falls back to an alternate
/// reference axis when `forward` is nearly parallel to world up (gimbal lock),
/// so the basis is always finite and orthonormal.
fn screen_basis(forward: Vector3<f32>) -> (Vector3<f32>, Vector3<f32>) {
    let world_up = Vector3::new(0.0, 1.0, 0.0);
    let right_raw = forward.cross(&world_up);
    let right = if right_raw.norm_squared() < 1e-6 {
        // Camera looks straight up or down — pick an arbitrary perpendicular.
        forward.cross(&Vector3::new(0.0, 0.0, 1.0)).normalize()
    } else {
        right_raw.normalize()
    };
    let up = right.cross(&forward).normalize();
    (right, up)
}

/// Project a world-space point to screen-space coordinates.
/// `screen` is the rect the engine projects the 3D scene into — in this editor
/// that is the full egui screen, since the engine renders to the OS window with
/// projection aspect = `camera.width / camera.height`. Passing a panel sub-rect
/// here makes the result diverge from the rendered scene whenever the panel's
/// aspect or position differs from the window's. Returns `None` if the point is
/// behind the camera (view_z <= 0).
pub fn world_to_screen(camera: &Camera, screen: Rect, world: Vector3<f32>) -> Option<Pos2> {
    let cam_pos = Vector3::from(camera.get_position());
    let forward = Vector3::from(camera.calculate_direction_vector());
    let (right, up) = screen_basis(forward);

    let rel = world - cam_pos;
    let view_z = rel.dot(&forward);
    if view_z <= 0.0 {
        return None;
    }
    let view_x = rel.dot(&right);
    let view_y = rel.dot(&up);

    let aspect = camera.width / camera.height;
    let half_h = (camera.fov / 2.0).tan() * view_z;
    let half_w = half_h * aspect;

    let ndc_x = view_x / half_w;
    let ndc_y = view_y / half_h;

    let screen_x = screen.min.x + (ndc_x + 1.0) * 0.5 * screen.width();
    let screen_y = screen.min.y + (1.0 - ndc_y) * 0.5 * screen.height();
    Some(Pos2::new(screen_x, screen_y))
}

/// Project a world-space line segment to screen-space, clipping to the
/// camera's near plane so that segments with one endpoint behind the camera
/// still render their visible portion. Returns `None` if both endpoints are
/// behind the near plane.
pub fn world_segment_to_screen(
    camera: &Camera,
    screen: Rect,
    a: Vector3<f32>,
    b: Vector3<f32>,
) -> Option<(Pos2, Pos2)> {
    const NEAR: f32 = 0.05;
    let cam_pos = Vector3::from(camera.get_position());
    let forward = Vector3::from(camera.calculate_direction_vector());
    let za = (a - cam_pos).dot(&forward);
    let zb = (b - cam_pos).dot(&forward);
    if za < NEAR && zb < NEAR {
        return None;
    }
    let (a_clipped, b_clipped) = if za >= NEAR && zb >= NEAR {
        (a, b)
    } else if za < NEAR {
        let t = (NEAR - za) / (zb - za);
        (a + (b - a) * t, b)
    } else {
        let t = (NEAR - zb) / (za - zb);
        (a, b + (a - b) * t)
    };
    let a_s = world_to_screen(camera, screen, a_clipped)?;
    let b_s = world_to_screen(camera, screen, b_clipped)?;
    Some((a_s, b_s))
}

/// Convert a screen-space cursor position to a world-space ray.
/// `screen` must be the same rect the engine projects into (see
/// `world_to_screen`) so the unprojection inverts the same mapping the renderer
/// used. Returns `(origin, direction)` with `direction` normalized.
pub fn unproject(camera: &Camera, screen_pos: Pos2, screen: Rect) -> (Vector3<f32>, Vector3<f32>) {
    let ndc_x = (screen_pos.x - screen.min.x) / screen.width() * 2.0 - 1.0;
    let ndc_y = -((screen_pos.y - screen.min.y) / screen.height() * 2.0 - 1.0);

    let aspect = camera.width / camera.height;
    let half_h = (camera.fov / 2.0).tan();
    let half_w = half_h * aspect;

    let forward = Vector3::from(camera.calculate_direction_vector());
    let (right, up) = screen_basis(forward);

    let dir = (forward + right * (ndc_x * half_w) + up * (ndc_y * half_h)).normalize();
    let origin = Vector3::from(camera.get_position());
    (origin, dir)
}

/// Closest point on the infinite line `line_origin + t * line_dir` to the ray
/// `ray_origin + s * ray_dir`. Both directions must be unit length.
pub fn closest_point_on_line_to_ray(
    line_origin: Vector3<f32>, line_dir: Vector3<f32>,
    ray_origin: Vector3<f32>, ray_dir: Vector3<f32>,
) -> Vector3<f32> {
    let w0 = line_origin - ray_origin;
    let a = line_dir.dot(&line_dir);
    let b = line_dir.dot(&ray_dir);
    let c = ray_dir.dot(&ray_dir);
    let d = line_dir.dot(&w0);
    let e = ray_dir.dot(&w0);
    let denom = a * c - b * b;
    let t = if denom.abs() < 1e-6 {
        // Lines are parallel: project ray_origin onto the line so the result
        // is the nearest point on the line to the ray origin (assumes line_dir
        // is unit length, which is the documented precondition).
        (ray_origin - line_origin).dot(&line_dir)
    } else {
        (b * e - c * d) / denom
    };
    line_origin + line_dir * t
}

/// Intersect a ray with a plane. Returns `None` if the ray is parallel to the plane.
pub fn ray_plane_intersect(
    ray_origin: Vector3<f32>, ray_dir: Vector3<f32>,
    plane_point: Vector3<f32>, plane_normal: Vector3<f32>,
) -> Option<Vector3<f32>> {
    let denom = plane_normal.dot(&ray_dir);
    if denom.abs() < 1e-6 {
        return None;
    }
    let t = (plane_point - ray_origin).dot(&plane_normal) / denom;
    Some(ray_origin + ray_dir * t)
}

/// 2D distance from point `p` to the segment between `a` and `b`.
pub fn distance_point_to_segment_2d(p: Pos2, a: Pos2, b: Pos2) -> f32 {
    let ab = b - a;
    let len_sq = ab.x * ab.x + ab.y * ab.y;
    if len_sq < 1e-6 {
        return (p - a).length();
    }
    let t = (((p - a).x * ab.x) + ((p - a).y * ab.y)) / len_sq;
    let t = t.clamp(0.0, 1.0);
    let foot = a + ab * t;
    (p - foot).length()
}

/// Round `value` to the nearest multiple of `step`.
pub fn snap(value: f32, step: f32) -> f32 {
    if step <= 0.0 { return value; }
    (value / step).round() * step
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_camera() -> Camera {
        // Place the camera at (0,0,-5). The engine's default forward, with
        // rotation = (0,0,0), is whatever `calculate_direction_vector` returns;
        // the tests below sample that direction rather than assume +Z or -Z.
        let mut cam = Camera::default();
        cam.set_position([0.0, 0.0, -5.0]);
        cam.set_rotation([0.0, 0.0, 0.0]);
        cam.width = 800.0;
        cam.height = 600.0;
        cam.fov = std::f32::consts::FRAC_PI_2; // 90°
        cam.near = 0.1;
        cam.far = 1000.0;
        cam
    }

    #[test]
    fn world_to_screen_point_in_front_lands_inside_rect() {
        let cam = test_camera();
        let rect = Rect::from_min_size(Pos2::new(0.0, 0.0), egui::Vec2::new(800.0, 600.0));
        let forward = Vector3::from(cam.calculate_direction_vector());
        // A point along the camera's forward direction should land near rect center.
        let target = Vector3::from(cam.get_position()) + forward * 5.0;
        let screen = world_to_screen(&cam, rect, target).expect("point in front");
        assert!((screen.x - 400.0).abs() < 1.0, "screen.x = {}", screen.x);
        assert!((screen.y - 300.0).abs() < 1.0, "screen.y = {}", screen.y);
    }

    #[test]
    fn world_to_screen_point_behind_camera_is_none() {
        let cam = test_camera();
        let rect = Rect::from_min_size(Pos2::new(0.0, 0.0), egui::Vec2::new(800.0, 600.0));
        let backward = -Vector3::from(cam.calculate_direction_vector());
        let behind = Vector3::from(cam.get_position()) + backward * 5.0;
        assert!(world_to_screen(&cam, rect, behind).is_none());
    }

    #[test]
    fn unproject_center_returns_forward_ray() {
        let cam = test_camera();
        let rect = Rect::from_min_size(Pos2::new(0.0, 0.0), egui::Vec2::new(800.0, 600.0));
        let center = Pos2::new(400.0, 300.0);
        let (origin, dir) = unproject(&cam, center, rect);
        let expected_forward = Vector3::from(cam.calculate_direction_vector());
        let expected_origin = Vector3::from(cam.get_position());
        assert!((origin - expected_origin).norm() < 1e-5);
        assert!((dir - expected_forward).norm() < 1e-5, "dir = {:?}", dir);
    }

    #[test]
    fn world_to_screen_camera_straight_down_does_not_nan() {
        let mut cam = Camera::default();
        cam.set_position([0.0, 5.0, 0.0]);
        // Rotation that points the camera straight down. The engine's
        // `calculate_direction_vector` returns
        // [-sin(yaw)*cos(pitch), sin(pitch), -cos(yaw)*cos(pitch)],
        // so pitch = -π/2 with yaw = 0 gives forward = [0, -1, 0].
        cam.set_rotation([-std::f32::consts::FRAC_PI_2, 0.0, 0.0]);
        cam.width = 800.0;
        cam.height = 600.0;
        cam.fov = std::f32::consts::FRAC_PI_2;
        cam.near = 0.1;
        cam.far = 1000.0;

        let rect = Rect::from_min_size(Pos2::new(0.0, 0.0), egui::Vec2::new(800.0, 600.0));
        // Point on the ground, directly below the camera — should project to center.
        let target = Vector3::new(0.0, 0.0, 0.0);
        let screen = world_to_screen(&cam, rect, target).expect("point in front");
        assert!(screen.x.is_finite(), "screen.x is not finite: {}", screen.x);
        assert!(screen.y.is_finite(), "screen.y is not finite: {}", screen.y);
    }

    #[test]
    fn closest_point_perpendicular_ray_hits_line_origin() {
        // Line along +X through origin. Ray comes straight down from (0, 1, 0).
        let p = closest_point_on_line_to_ray(
            Vector3::new(0.0, 0.0, 0.0), Vector3::new(1.0, 0.0, 0.0),
            Vector3::new(0.0, 1.0, 0.0), Vector3::new(0.0, -1.0, 0.0),
        );
        assert!((p - Vector3::zeros()).norm() < 1e-4, "p = {:?}", p);
    }

    #[test]
    fn ray_plane_intersect_hits_xy_plane() {
        let p = ray_plane_intersect(
            Vector3::new(0.0, 0.0, 1.0), Vector3::new(0.0, 0.0, -1.0),
            Vector3::zeros(), Vector3::new(0.0, 0.0, 1.0),
        ).expect("ray hits plane");
        assert!((p - Vector3::zeros()).norm() < 1e-4);
    }

    #[test]
    fn ray_plane_intersect_parallel_returns_none() {
        let r = ray_plane_intersect(
            Vector3::new(0.0, 0.0, 1.0), Vector3::new(1.0, 0.0, 0.0),
            Vector3::zeros(), Vector3::new(0.0, 0.0, 1.0),
        );
        assert!(r.is_none());
    }

    #[test]
    fn distance_point_to_segment_perpendicular_foot_inside() {
        let d = distance_point_to_segment_2d(
            Pos2::new(5.0, 3.0),
            Pos2::new(0.0, 0.0),
            Pos2::new(10.0, 0.0),
        );
        assert!((d - 3.0).abs() < 1e-4);
    }

    #[test]
    fn distance_point_to_segment_before_a() {
        let d = distance_point_to_segment_2d(
            Pos2::new(-3.0, 0.0),
            Pos2::new(0.0, 0.0),
            Pos2::new(10.0, 0.0),
        );
        assert!((d - 3.0).abs() < 1e-4);
    }

    #[test]
    fn distance_point_to_segment_after_b() {
        let d = distance_point_to_segment_2d(
            Pos2::new(13.0, 0.0),
            Pos2::new(0.0, 0.0),
            Pos2::new(10.0, 0.0),
        );
        assert!((d - 3.0).abs() < 1e-4);
    }

    #[test]
    fn snap_translate_step_one() {
        assert!((snap(2.3, 1.0) - 2.0).abs() < 1e-6);
        assert!((snap(-2.7, 1.0) - -3.0).abs() < 1e-6);
    }

    #[test]
    fn snap_angle_step_fifteen_degrees() {
        // 15° = π/12 ≈ 0.2618; 0.5 rad ≈ 1.909 steps → rounds to 2 → result = π/6.
        let step = std::f32::consts::PI / 12.0;
        let expected = std::f32::consts::PI / 6.0;
        assert!((snap(0.5, step) - expected).abs() < 1e-5);
    }

    #[test]
    fn closest_point_parallel_lines_projects_ray_origin() {
        // Line along +X through (1, 0, 0). Ray along +X starting at (0, 1, 0).
        // Lines are parallel; expected result is the foot of perpendicular from
        // ray_origin onto the line: project (0,1,0) - (1,0,0) onto +X → t = -1 →
        // (1,0,0) + (-1)*(1,0,0) = (0,0,0).
        let p = closest_point_on_line_to_ray(
            Vector3::new(1.0, 0.0, 0.0), Vector3::new(1.0, 0.0, 0.0),
            Vector3::new(0.0, 1.0, 0.0), Vector3::new(1.0, 0.0, 0.0),
        );
        assert!((p - Vector3::zeros()).norm() < 1e-4, "p = {:?}", p);
    }

    #[test]
    fn snap_zero_or_negative_step_returns_value_unchanged() {
        assert!((snap(3.7, 0.0) - 3.7).abs() < 1e-6);
        assert!((snap(3.7, -1.0) - 3.7).abs() < 1e-6);
    }
}
