fn main() {
    prost_build::compile_protos(
        &[
            "src/proto/hearthstone/game_state.proto",
        ],
        &["src/proto"],
    ).unwrap();

    prost_build::compile_protos(
        &[
            "src/proto/csgo/cstrike15_gcmessages.proto",
            "src/proto/csgo/cstrike15_usermessages.proto",
            "src/proto/csgo/engine_gcmessages.proto",
            "src/proto/csgo/netmessages.proto",
            "src/proto/csgo/steammessages.proto",
        ],
        &["src/proto/csgo"],
    ).unwrap();
}