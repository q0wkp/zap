use super::is_zap_ng_bundle;

#[test]
fn is_zap_ng_bundle_recognises_zap_ng_channels() {
    // OSS (ZapNG) 自身。
    assert!(is_zap_ng_bundle("dev.q0wkp.zapng"));
    // 上游 Warp 各 channel —— 同样视为本应用家族,允许 default-app 重定向。
    assert!(is_zap_ng_bundle("dev.warp.ZapNG"));
    assert!(is_zap_ng_bundle("dev.warp.WarpDev"));
    assert!(is_zap_ng_bundle("dev.warp.WarpPreview"));
    assert!(is_zap_ng_bundle("dev.warp.WarpOss"));
}

#[test]
fn is_zap_ng_bundle_rejects_other_apps() {
    assert!(!is_zap_ng_bundle("com.microsoft.VSCode"));
    assert!(!is_zap_ng_bundle("com.apple.TextEdit"));
    assert!(!is_zap_ng_bundle("dev.zed.Zed"));
    assert!(!is_zap_ng_bundle("invalid"));
    assert!(!is_zap_ng_bundle(""));
}
