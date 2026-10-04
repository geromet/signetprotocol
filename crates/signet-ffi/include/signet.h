/*
 * Signet Protocol — C interface of the Signet SDK (beta). Apache-2.0.
 *
 * A complete client in about ten calls: connect, read the world and the
 * bodies, set the player's intent and advance every frame (prediction and
 * reconciliation included). Nothing blocks.
 *
 *   SgClient *c = sg_connect("192.168.1.212", "my-game");
 *   while (playing) {
 *       sg_intent(c, forward, strafe, yaw, run, fire, SG_WEAPON_PISTOL);
 *       float x, y, z;
 *       if (sg_advance(c, dt, &x, &y, &z)) place_camera(x, y + 1.3f, z, yaw);
 *   }
 *   sg_free(c);
 *
 * Neutral coordinates: metres; x east, y up, z south. yaw in radians,
 * 0 looks towards -z and positive values turn left.
 */
#ifndef SIGNET_H
#define SIGNET_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct SgClient SgClient;

enum { SG_CONNECTING = 0, SG_CONNECTED = 1, SG_DISCONNECTED = 2 };
enum { SG_WEAPON_FIST = 0, SG_WEAPON_PISTOL = 1, SG_WEAPON_SHOTGUN = 2 };

/* A body in the latest server snapshot (bots included). */
typedef struct SgPlayer {
    uint32_t id;
    float x, y, z;   /* feet */
    float yaw;
    int32_t health;  /* 0–100 */
    int32_t ammo;
    int32_t frags;
    uint8_t weapon;
    uint8_t alive;
    char game[16];   /* "doom", "minecraft", "bot"… NUL-terminated */
} SgPlayer;

/* SDK version ("0.1.0-beta.1"). */
const char *sg_version(void);

/* Connects to host ("192.168.1.212" or "host:port") as game ("observador"
 * to only watch). Retries in the background. NULL if the arguments are not
 * valid text. */
SgClient *sg_connect(const char *host, const char *game);
void sg_free(SgClient *c);
int32_t sg_state(const SgClient *c);
/* Id of your own body (0 = none yet). */
uint32_t sg_my_id(const SgClient *c);

/* Terrain: a grid of 1 m cells centred on the origin (0 = not arrived yet). */
int32_t sg_terrain_side(const SgClient *c);
float sg_terrain_height(const SgClient *c, float x, float z);
int32_t sg_terrain_walkable(const SgClient *c, float x, float z);
/* The whole terrain as Signet/1 JSON. Returns the size needed without the
 * NUL; if cap is too small the output is truncated (call again with more). */
size_t sg_terrain_json(const SgClient *c, char *buf, size_t cap);
/* The native Signet map (lights, objects, sky) as JSON; 0 if the world is not native. */
size_t sg_native_map_json(const SgClient *c, char *buf, size_t cap);

/* Copies up to max bodies and returns how many there are. */
int32_t sg_players(const SgClient *c, SgPlayer *out, int32_t max);
/* Events since the last call as a JSON array (Disparo, Danio, Muerte,
 * Reaparicion). Consumes them: use a large buffer (64 KB). */
size_t sg_events_json(const SgClient *c, char *buf, size_t cap);

/* What the player wants to do right now. */
void sg_intent(SgClient *c, float forward, float strafe, float yaw, int32_t run, int32_t fire, uint8_t weapon);
/* Advances dt seconds: predicts, sends the commands for the elapsed ticks and
 * writes where to draw the feet. 1 if there is a body, 0 if not yet. */
int32_t sg_advance(SgClient *c, float dt, float *x, float *y, float *z);
/* After a (re)spawn, the best direction to look at (once). 1 if there is a value. */
int32_t sg_spawn_yaw(SgClient *c, float *yaw);
/* Prediction corrections since connecting (0 = everything agrees). */
uint32_t sg_corrections(const SgClient *c);

#ifdef __cplusplus
}
#endif

#endif
