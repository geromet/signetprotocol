/* Minimal C client: joins, walks in a circle for 5 seconds and prints its position.
 *
 *   Windows (MSVC):  cl client.c /I ..\..\crates\signet-ffi\include ..\..\target\release\signet.dll.lib
 *   Linux / macOS:   cc client.c -I ../../crates/signet-ffi/include -L ../../target/release -lsignet
 */
#include <stdio.h>
#include "signet.h"

#ifdef _WIN32
#include <windows.h>
static void sleep_ms(int ms) { Sleep(ms); }
#else
#include <unistd.h>
static void sleep_ms(int ms) { usleep(ms * 1000); }
#endif

int main(int argc, char **argv) {
    const char *host = argc > 1 ? argv[1] : "127.0.0.1";
    printf("Signet SDK %s\n", sg_version());
    SgClient *c = sg_connect(host, "sdk-c");
    float yaw = 0.0f;
    for (int f = 0; f < 5 * 60; f++) {
        float x, y, z, spawn;
        if (sg_spawn_yaw(c, &spawn)) yaw = spawn;
        yaw += 0.5f / 60.0f;
        sg_intent(c, 1.0f, 0.0f, yaw, 0, 0, SG_WEAPON_PISTOL);
        int has_body = sg_advance(c, 1.0f / 60.0f, &x, &y, &z);
        if (f % 60 == 0) {
            SgPlayer players[64];
            int n = sg_players(c, players, 64);
            if (has_body)
                printf("pos (%.2f, %.2f, %.2f) | %d bodies | terrain %d m | corrections %u\n", x, y, z, n, sg_terrain_side(c), sg_corrections(c));
            else
                printf("state %d: waiting for a body...\n", sg_state(c));
        }
        sleep_ms(16);
    }
    sg_free(c);
    return 0;
}
