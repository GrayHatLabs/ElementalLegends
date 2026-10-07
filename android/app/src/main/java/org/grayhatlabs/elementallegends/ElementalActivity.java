package org.grayhatlabs.elementallegends;

import org.libsdl.app.SDLActivity;

/** Elemental Legends: SDL runs the game from libmain.so (SDL_main in src/main.rs). */
public class ElementalActivity extends SDLActivity {
    @Override
    protected String[] getLibraries() {
        return new String[] { "SDL2", "main" };
    }
}
