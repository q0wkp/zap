// Our class for handling NSServices messages.
@interface ZapNGServicesProvider : NSObject
@end

// Functions implemented in Rust.
id zap_ng_services_provider_custom_url_scheme();
void zap_ng_app_open_urls(id app, id urls);
