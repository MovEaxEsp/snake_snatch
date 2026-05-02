// Helper JS shared between index.html and dev.html


import init, { run_frame, init_state, default_config, update_config } from './pkg/snake_snatch.js';

export function default_cfg() {
  return default_config();
}

export function update_cfg(new_config) {
  return update_config(new_config);
}

// List out all resources to be loaded here
const resources = {images: [], sounds: []};

const images = {};
const sounds = {};
const audioCtx = new AudioContext();


// Load all the resources that will be used by the game.  This must be done before init_game can be called
export async function load_resources() {

  // 'init' allows us to call the other rust-wasm functions
  const promises = [init()];

  resources.images.forEach(name => {
    let img = new Image()

    let imgPromise = new Promise((resolve, _) => {
      img.onload = resolve;
    });
    promises.push(imgPromise);

    img.src = `./images/${name}`;
    images[name] = img;
  });

  resources.sounds.forEach(name => {
    promises.push(
      window.fetch(`./audio/${name}`).then(async (response) => {
        if (!response.ok) {
          throw new Error(`Failed loading ${name}`);
        }

        let arrayBuf = await response.arrayBuffer();
        sounds[name] = await audioCtx.decodeAudioData(arrayBuf);
      })
    );
  });

  return Promise.all(promises);
}

let run_loop_started = false;

export function init_game(canvas) {
  let gameConfig = {...default_config(), ...{ }};
  init_state(gameConfig, canvas, images, audioCtx, sounds);

  if (!run_loop_started) {
    run_loop_started = true;
    setInterval(function() { run_frame(); }, 1000/30);
  }
}
