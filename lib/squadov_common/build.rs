fn main() {
    prost_build::compile_protos(&["src/proto/hearthstone/game_state.proto"],
                                &["src/proto"]).unwrap();
}