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

/// Project a world-space point to screen-space coordinates inside `rect`.
/// Returns `None` if the point is behind the camera (view_z <= 0).
pub fn world_to_screen(camera: &Camera, rect: Rect, world: Vector3<f32>) -> Option<Pos2> {
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

    let screen_x = rect.min.x + (ndc_x + 1.0) * 0.5 * rect.width();
    let screen_y = rect.min.y + (1.0 - ndc_y) * 0.5 * rect.height();
    Some(Pos2::new(screen_x, screen_y))
}

/// Convert a screen-space cursor position to a world-space ray.
/// Returns `(origin, direction)` with `direction` normalized.
pub fn unproject(camera: &Camera, screen_pos: Pos2, rect: Rect) -> (Vector3<f32>, Vector3<f32>) {
    let ndc_x = (screen_pos.x - rect.min.x) / rect.width() * 2.0 - 1.0;
    let ndc_y = -((screen_pos.y - rect.min.y) / rect.height() * 2.0 - 1.0);

    let aspect = camera.width / camera.height;
    let half_h = (camera.fov / 2.0).tan();
    let half_w = half_h * aspect;

    let forward = Vector3::from(camera.calculate_direction_vector());
    let (right, up) = screen_basis(forward);

    let dir = (forward + right * (ndc_x * half_w) + up * (ndc_y * half_h)).normalize();
    let origin = Vector3::from(camera.get_position());
    (origin, dir)
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
}
