use cfg_aliases::cfg_aliases;

fn main() {
    // This sets the same cfg aliases as the `warp` crate, used to gate crash-recovery flags.
    cfg_aliases! {
        enable_crash_recovery: { target_os = "linux" },
    }
}
