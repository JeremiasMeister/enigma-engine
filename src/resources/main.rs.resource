fn main(){
    // create a new enigma app_state
    let mut app_state = enigma::AppState::new();

    // create a new enigma event loop, providing a window title and dimensions
    let event_loop = enigma::EventLoop::new("ENIGMA_PROJECT_NAME", 1080, 720); // change the title and dimensions to your liking

    // create a new camera, and set it in the app_state. Each enigma project must have a camera
    let camera = enigma::camera::Camera::new(None, None, Some(60.0), Some(16.0/9.0), Some(0.1), Some(100.0));
    app_state.set_camera(camera);

    // run the event loop, providing the app_state as an arc mutex
    event_loop.run(app_state.convert_to_arc_mutex());
}