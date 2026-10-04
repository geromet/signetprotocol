// Signet Protocol — C# binding (preview) over the C interface (signet.dll / libsignet.so).
// For Unity (copy this file and the native library into Assets/Plugins) and .NET.
// Apache-2.0.

using System;
using System.Runtime.InteropServices;
using System.Text;

namespace Signet
{
    public enum ConnectionState { Connecting = 0, Connected = 1, Disconnected = 2 }

    public enum Weapon : byte { Fist = 0, Pistol = 1, Shotgun = 2 }

    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Ansi)]
    public struct Player
    {
        public uint Id;
        public float X, Y, Z;
        public float Yaw;
        public int Health;
        public int Ammo;
        public int Frags;
        public byte Weapon;
        public byte Alive;
        [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 16)]
        public string Game;
    }

    internal static class Native
    {
        const string Lib = "signet";

        [DllImport(Lib)] public static extern IntPtr sg_version();
        [DllImport(Lib)] public static extern IntPtr sg_connect(string host, string game);
        [DllImport(Lib)] public static extern void sg_free(IntPtr c);
        [DllImport(Lib)] public static extern int sg_state(IntPtr c);
        [DllImport(Lib)] public static extern uint sg_my_id(IntPtr c);
        [DllImport(Lib)] public static extern int sg_terrain_side(IntPtr c);
        [DllImport(Lib)] public static extern float sg_terrain_height(IntPtr c, float x, float z);
        [DllImport(Lib)] public static extern int sg_terrain_walkable(IntPtr c, float x, float z);
        [DllImport(Lib)] public static extern UIntPtr sg_terrain_json(IntPtr c, byte[] buf, UIntPtr cap);
        [DllImport(Lib)] public static extern UIntPtr sg_native_map_json(IntPtr c, byte[] buf, UIntPtr cap);
        [DllImport(Lib)] public static extern int sg_players(IntPtr c, [Out] Player[] output, int max);
        [DllImport(Lib)] public static extern UIntPtr sg_events_json(IntPtr c, byte[] buf, UIntPtr cap);
        [DllImport(Lib)] public static extern void sg_intent(IntPtr c, float forward, float strafe, float yaw, int run, int fire, byte weapon);
        [DllImport(Lib)] public static extern int sg_advance(IntPtr c, float dt, out float x, out float y, out float z);
        [DllImport(Lib)] public static extern int sg_spawn_yaw(IntPtr c, out float yaw);
        [DllImport(Lib)] public static extern uint sg_corrections(IntPtr c);
    }

    /// <summary>Connection to a Signet server, prediction included.</summary>
    public sealed class Client : IDisposable
    {
        IntPtr c;
        readonly Player[] playerBuffer = new Player[128];
        readonly byte[] eventBuffer = new byte[65536];

        public static string SdkVersion => Marshal.PtrToStringAnsi(Native.sg_version());

        public Client(string host, string game) { c = Native.sg_connect(host, game); }

        public ConnectionState State => (ConnectionState)Native.sg_state(c);
        public uint MyId => Native.sg_my_id(c);
        public int TerrainSide => Native.sg_terrain_side(c);
        public float Height(float x, float z) => Native.sg_terrain_height(c, x, z);
        public bool Walkable(float x, float z) => Native.sg_terrain_walkable(c, x, z) != 0;
        public uint Corrections => Native.sg_corrections(c);

        /// <summary>The whole terrain as Signet/1 JSON (null if it has not arrived yet).</summary>
        public string TerrainJson() => ReadJson(Native.sg_terrain_json);

        /// <summary>The native Signet map as JSON (null if the world is not native).</summary>
        public string NativeMapJson() => ReadJson(Native.sg_native_map_json);

        string ReadJson(Func<IntPtr, byte[], UIntPtr, UIntPtr> read)
        {
            var n = (int)read(c, null, UIntPtr.Zero);
            if (n == 0) return null;
            var buf = new byte[n + 1];
            read(c, buf, (UIntPtr)buf.Length);
            return Encoding.UTF8.GetString(buf, 0, n);
        }

        public ArraySegment<Player> Players()
        {
            var n = Math.Min(Native.sg_players(c, playerBuffer, playerBuffer.Length), playerBuffer.Length);
            return new ArraySegment<Player>(playerBuffer, 0, n);
        }

        /// <summary>Events since the last call, as a JSON array.</summary>
        public string TakeEventsJson()
        {
            var n = (int)Native.sg_events_json(c, eventBuffer, (UIntPtr)eventBuffer.Length);
            return Encoding.UTF8.GetString(eventBuffer, 0, Math.Min(n, eventBuffer.Length - 1));
        }

        public void Intent(float forward, float strafe, float yaw, bool run, bool fire, Weapon weapon) =>
            Native.sg_intent(c, forward, strafe, yaw, run ? 1 : 0, fire ? 1 : 0, (byte)weapon);

        /// <summary>Advances dt seconds; returns where to draw the feet, or false if there is no body yet.</summary>
        public bool Advance(float dt, out float x, out float y, out float z) => Native.sg_advance(c, dt, out x, out y, out z) != 0;

        public bool SpawnYaw(out float yaw) => Native.sg_spawn_yaw(c, out yaw) != 0;

        public void Dispose()
        {
            if (c != IntPtr.Zero) { Native.sg_free(c); c = IntPtr.Zero; }
        }
    }
}
