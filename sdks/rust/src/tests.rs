use super::*;
use std::{io::{Read, Write}, net::{Ipv4Addr, TcpListener}, thread};

struct Step { method: &'static str, path: &'static str, response: String, headers: Vec<(&'static str, String)> }

fn json_step(method: &'static str, path: &'static str, value: Value) -> Step {
    let body = serde_json::to_string(&value).unwrap();
    Step { method, path, response: format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()), headers: Vec::new() }
}

fn server(steps: Vec<Step>) -> (Client, thread::JoinHandle<()>) {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
    listener.set_nonblocking(true).unwrap();
    let address = listener.local_addr().unwrap();
    let task = thread::spawn(move || {
        for step in steps {
            let deadline = std::time::Instant::now() + Duration::from_secs(10);
            let (mut stream, _) = loop {
                match listener.accept() {
                    Ok(pair) => break pair,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock && std::time::Instant::now() < deadline => thread::sleep(Duration::from_millis(5)),
                    Err(_) => panic!("expected request deadline"),
                }
            };
            stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
            let mut raw = Vec::new();
            let mut chunk = [0; 1024];
            loop {
                let count = stream.read(&mut chunk).unwrap();
                assert!(count > 0, "unexpected request EOF");
                raw.extend_from_slice(&chunk[..count]);
                if let Some(end) = raw.windows(4).position(|x| x == b"\r\n\r\n") {
                    let head = String::from_utf8(raw[..end].to_vec()).unwrap();
                    let length = head.lines().find_map(|line| line.to_ascii_lowercase().strip_prefix("content-length:").map(|value| value.trim().parse::<usize>().unwrap())).unwrap_or(0);
                    if raw.len() >= end + 4 + length { break; }
                }
                assert!(raw.len() < 64 * 1024);
            }
            let raw = String::from_utf8(raw).unwrap();
            let head = raw.split("\r\n\r\n").next().unwrap();
            assert!(head.starts_with(&format!("{} {} HTTP/1.1\r\n", step.method, step.path)));
            // A step names its exact encoded query when it expects one; otherwise none is sent.
            if !step.path.contains('?') { assert!(!head.lines().next().unwrap().contains('?')); }
            if !step.headers.iter().any(|(name,_)|name.eq_ignore_ascii_case("sec-purpose")) {
                assert!(!head.to_ascii_lowercase().contains("\r\nsec-purpose:"));
            }
            for (name, value) in &step.headers {
                let expected = format!("{}: {}", name.to_ascii_lowercase(), value);
                assert!(head.lines().any(|line| line.eq_ignore_ascii_case(&expected)), "required header missing");
            }
            if step.path.ends_with("/stream") || step.path == "/nimbus-play/v1/sxapi" {
                assert!(!head.to_ascii_lowercase().contains("authorization:"));
            }
            stream.write_all(step.response.as_bytes()).unwrap();
        }
    });
    (Client::build(&format!("http://{address}"), identity(), true).unwrap(), task)
}

fn build_info() -> BuildInfo {
    BuildInfo { product:"ExamplePlayer".into(),version:"1.0.0".into(),system:"TestOS".into(),system_version:"1.0".into(),model_or_arch:"x64".into(),build:"fixture".into(),channel:"test".into(),streamix_core_version:None }
}

fn identity() -> RequestIdentity {
    RequestIdentity::new(build_info(),DeviceIdentity { installation_id:Uuid::parse_str("55555555-5555-4555-8555-555555555555").unwrap(),display_name:"Living room".into() },Some("en")).unwrap()
}

fn discovery() -> Step {
    json_step("GET", "/sxapi/v1/capabilities", serde_json::json!({"protocol":"SXAPI","version":"1.0","features":{"playback_ticket_header":true,"subtitle_ticket_header":true,"credentials_in_urls":false,"playback":true,"subtitles":true}}))
}

fn discovery_with(extra: &[&str]) -> Step {
    let mut features = serde_json::json!({"playback_ticket_header":true,"subtitle_ticket_header":true,"credentials_in_urls":false,"playback":true,"subtitles":true});
    for name in extra { features[*name] = Value::Bool(true); }
    json_step("GET", "/sxapi/v1/capabilities", serde_json::json!({"protocol":"SXAPI","version":"1.0","features":features}))
}

/// A synthetic rotating login refresh token, assembled so no token literal is in the source.
fn rotating(digit: char) -> String { format!("stx_sxls_{}", digit.to_string().repeat(64)) }

fn instant(value: &str) -> DateTime<Utc> { value.parse().unwrap() }

fn parameter(document: &Value, path: &str, name: &str) -> Value {
    document["paths"][path]["get"]["parameters"].as_array().unwrap().iter().find(|item| item["name"] == name).cloned().unwrap()
}

fn login_step() -> Step {
    json_step("POST", "/sxapi/v1/sessions", serde_json::json!({"access_token": (["stx_sx_", "fixture"].concat()),"token_type":"Bearer","user":{"id":"11111111-1111-4111-8111-111111111111","username":"fixture","library_ids":[]}}))
}

fn prepared() -> Step {
    prepared_path("/sxapi/v1/playback/sessions/22222222-2222-4222-8222-222222222222/stream", 100)
}

fn prepared_path(path: &str, seconds: i64) -> Step {
    let now = Utc::now();
    let deadline = (now + chrono::Duration::seconds(100)).to_rfc3339();
    let ticket_deadline = (now + chrono::Duration::seconds(seconds)).to_rfc3339();
    json_step("POST", "/sxapi/v1/playback/sessions", serde_json::json!({
        "session":{"session_id":"22222222-2222-4222-8222-222222222222","media_id":"33333333-3333-4333-8333-333333333333","episode_id":null,"version_id":"44444444-4444-4444-8444-444444444444","mode":"direct_play","compatibility":"verified","expires_at":deadline},
        "ticket":{"stream_path":path,"header_name":VIDEO_HEADER,"ticket":"fixture-video-ticket","expires_at":ticket_deadline}
    }))
}

fn end_step() -> Step {
    Step { method:"DELETE", path:"/sxapi/v1/playback/sessions/22222222-2222-4222-8222-222222222222", response:"HTTP/1.1 204 No Content\r\nConnection: close\r\n\r\n".into(), headers:vec![] }
}

fn playback_request() -> PlaybackRequest {
    PlaybackRequest { media_id:Uuid::parse_str("33333333-3333-4333-8333-333333333333").unwrap(), episode_id:None, version_id:None, profile:PlaybackProfile::default(), device_id:None }
}

#[test]
fn secrets_and_errors_are_redacted() {
    let secret = Secret::new("not-for-output".into());
    assert_eq!(format!("{secret:?}"), "Secret([REDACTED])");
    assert_eq!(Error::Transport.to_string(), "StreamixKit Transport");
}

#[test]
fn identity_is_stable_but_trace_and_purpose_are_per_request() {
    let value = identity();
    let interaction = value.headers(RequestPurpose::Interaction);
    let prefetch = value.headers(RequestPurpose::Prefetch);
    assert_eq!(interaction.get(header::USER_AGENT),prefetch.get(header::USER_AGENT));
    assert!(!interaction.contains_key("sec-purpose"));
    assert_eq!(prefetch.get("sec-purpose").unwrap(),"prefetch");
    assert_eq!(interaction.get("x-streamix-protocol").unwrap(),"SXAPI/1.0");
    assert_ne!(interaction.get("traceparent"),prefetch.get("traceparent"));
    for headers in [interaction,prefetch] {
        let trace = headers.get("traceparent").unwrap().to_str().unwrap();
        let parts: Vec<_> = trace.split('-').collect();
        assert_eq!(parts.iter().map(|p|p.len()).collect::<Vec<_>>(),vec![2,32,16,2]);
        assert_eq!(parts[0],"00");
        assert_eq!(parts[3],"01");
        assert!(!headers.contains_key(header::AUTHORIZATION));
    }
}

#[test]
fn utf8_device_name_is_bounded_before_percent_encoding() {
    let id = Uuid::new_v4();
    let identity = RequestIdentity::new(build_info(),DeviceIdentity { installation_id:id,display_name:"客厅电视".into() },None).unwrap();
    assert_eq!(identity.headers(RequestPurpose::Interaction).get("x-streamix-device-name").unwrap(),"%E5%AE%A2%E5%8E%85%E7%94%B5%E8%A7%86");
    assert!(matches!(RequestIdentity::new(build_info(),DeviceIdentity { installation_id:id,display_name:"影".repeat(43) },None),Err(Error::InvalidInput)));
    assert!(matches!(RequestIdentity::new(build_info(),DeviceIdentity { installation_id:id,display_name:"invalid\r\nname".into() },None),Err(Error::InvalidInput)));
}

#[test]
fn assembled_client_label_respects_the_server_byte_bound() {
    let mut build = build_info();
    build.product = "p".repeat(63);
    build.version = "v".repeat(64);
    let device = DeviceIdentity { installation_id:Uuid::new_v4(),display_name:"Fixture".into() };
    let accepted = RequestIdentity::new(build.clone(),device.clone(),None).unwrap();
    assert_eq!(accepted.headers(RequestPurpose::Interaction).get("x-streamix-client").unwrap().as_bytes().len(),128);
    build.product.push('p');
    assert!(matches!(RequestIdentity::new(build,device,None),Err(Error::InvalidInput)));
}

#[test]
fn retry_after_is_bounded_normalized_and_rejects_ambiguous_values() {
    let mut headers = header::HeaderMap::new();
    headers.insert(header::RETRY_AFTER,HeaderValue::from_static("120"));
    assert_eq!(parse_retry_after(&headers),Some(RetryAfter::DelaySeconds(120)));
    headers.insert(header::RETRY_AFTER,HeaderValue::from_static("00000000000000000000017"));
    assert_eq!(parse_retry_after(&headers),Some(RetryAfter::DelaySeconds(17)));
    headers.insert(header::RETRY_AFTER,HeaderValue::from_static("Wed, 21 Oct 2015 07:28:00 GMT"));
    assert_eq!(parse_retry_after(&headers),Some(RetryAfter::HttpDate(SystemTime::UNIX_EPOCH+Duration::from_secs(1_445_412_480))));
    for value in ["", "-1", "+1", "1.5", "18446744073709551616", "not-a-date", "Wed, 99 Oct 2015 07:28:00 GMT"] {
        headers.insert(header::RETRY_AFTER,HeaderValue::from_str(value).unwrap());
        assert_eq!(parse_retry_after(&headers),None);
    }
    headers.insert(header::RETRY_AFTER,HeaderValue::from_str(&"x".repeat(65)).unwrap());
    assert_eq!(parse_retry_after(&headers),None);
    headers.insert(header::RETRY_AFTER,HeaderValue::from_static("1"));
    headers.append(header::RETRY_AFTER,HeaderValue::from_static("2"));
    assert_eq!(parse_retry_after(&headers),None);
}

#[test]
fn installation_store_reuses_id_and_never_overwrites_invalid_data() {
    struct Directory(std::path::PathBuf);
    impl Drop for Directory { fn drop(&mut self) { let _ = std::fs::remove_dir_all(&self.0); } }
    let directory = Directory(std::env::temp_dir().join(format!("streamix-kit-fixture-{}",Uuid::new_v4())));
    std::fs::create_dir(&directory.0).unwrap();
    let path = directory.0.join("installation-id");
    let first = load_or_create_installation_id(&path).unwrap();
    assert_eq!(first.get_version_num(),4);
    assert_eq!(first,load_or_create_installation_id(&path).unwrap());
    std::fs::write(&path,b"invalid-existing-file").unwrap();
    assert!(matches!(load_or_create_installation_id(&path),Err(Error::InvalidInput)));
    assert_eq!(std::fs::read(&path).unwrap(),b"invalid-existing-file");
}

#[test]
fn typed_requests_match_the_synchronized_public_property_inventory() {
    let document: Value = serde_json::from_str(include_str!("../../../spec/sxapi-v1-client.openapi.json")).unwrap();
    let schemas = &document["components"]["schemas"];
    let request = serde_json::to_value(playback_request()).unwrap();
    for (name,value) in [("PlaybackRequest",request.clone()),("PlaybackProfile",request["profile"].clone()),("PlaybackProgress",serde_json::to_value(Progress { sequence:1,position_ms:0,completed:false }).unwrap())] {
        let properties = schemas[name]["properties"].as_object().unwrap();
        assert!(value.as_object().unwrap().keys().all(|key|properties.contains_key(key)));
        assert!(schemas[name]["required"].as_array().into_iter().flatten().all(|key|value.get(key.as_str().unwrap()).is_some()));
    }
    assert_eq!(schemas["Capabilities"]["properties"]["features"]["properties"]["playback_ticket_header"]["const"],true);
    assert_eq!(schemas["Capabilities"]["properties"]["features"]["properties"]["credentials_in_urls"]["const"],false);
}

#[test]
fn origin_and_path_guards() {
    for origin in ["http://streamix.example", "https://user:password@streamix.example", "https://streamix.example/sxapi/v1", "https://streamix.example/?x=1"] {
        assert!(matches!(Client::new(origin,identity()), Err(Error::InvalidOrigin)));
    }
    let client = Client::new("https://streamix.example",identity()).unwrap();
    for path in ["//other.example/a", "/../a", "/sxapi/v1/a?secret=value", "/sxapi/v1/%2e%2e/a", "https://other.example/a"] {
        assert!(matches!(client.inner.relative(path), Err(Error::InvalidPath)));
    }
    assert_eq!(client.inner.relative("/nimbus-play/v1/sxapi").unwrap().as_str(), "https://streamix.example/nimbus-play/v1/sxapi");
    assert!(!strong_etag("W/\"old\""));
    assert!(strong_etag("\"current\""));
}

#[test]
fn explicit_protocol_and_ports_never_downgrade() {
    assert_eq!(ServerAddress::parse("streamix.example",ConnectionMode::Https).unwrap().origin(),"https://streamix.example/");
    assert_eq!(ServerAddress::parse("media.local",ConnectionMode::LocalHttp).unwrap().origin(),"http://media.local:8970/");
    assert_eq!(ServerAddress::parse("http://media.local:80",ConnectionMode::LocalHttp).unwrap().origin(),"http://media.local/");
    assert_eq!(ServerAddress::parse("https://streamix.example:9443",ConnectionMode::Https).unwrap().origin(),"https://streamix.example:9443/");
    assert!(matches!(ServerAddress::parse("http://media.local",ConnectionMode::Https),Err(Error::InvalidOrigin)));
    assert!(matches!(ServerAddress::parse("https://streamix.example",ConnectionMode::LocalHttp),Err(Error::InvalidOrigin)));
    assert!(matches!(ServerAddress::parse("http://media.local:",ConnectionMode::LocalHttp),Err(Error::InvalidOrigin)));
    assert!(matches!(ServerAddress::parse("http://media.local:0",ConnectionMode::LocalHttp),Err(Error::InvalidOrigin)));
}

#[tokio::test]
async fn old_discovery_is_rejected_before_password() {
    let (client, task) = server(vec![json_step("GET", "/sxapi/v1/capabilities", serde_json::json!({"protocol":"SXAPI","version":"1.0","features":{"playback":true}}))]);
    let seen = Arc::new(std::sync::Mutex::new(Vec::new()));
    let capture = seen.clone();
    let client = client.with_request_observer(move |request|capture.lock().unwrap().push(request.clone()));
    let result = client.login("fixture", Secret::new("private-fixture".into())).await;
    assert!(matches!(result, Err(Error::UnsupportedHeaderContract)));
    assert_eq!(seen.lock().unwrap().len(),1);
    assert_eq!(seen.lock().unwrap()[0].method,"GET");
    task.join().unwrap();
}

#[tokio::test]
async fn login_redirect_is_not_followed() {
    let response = "HTTP/1.1 302 Found\r\nLocation: https://other.example/session\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".into();
    let (client, task) = server(vec![discovery(), Step { method:"POST", path:"/sxapi/v1/sessions", response, headers:vec![] }]);
    assert!(matches!(client.login("fixture", Secret::new("private-fixture".into())).await, Err(Error::Http { status:302, .. })));
    task.join().unwrap();
}

#[tokio::test]
async fn http_errors_retain_backoff_without_automatic_retry_or_raw_text() {
    let body = r#"{"error":{"code":"rate_limited","message":"private-body-sentinel"}}"#;
    let failed = Step { method:"POST",path:"/sxapi/v1/sessions",response:format!("HTTP/1.1 429 Too Many Requests\r\nRetry-After: 17\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()),headers:vec![] };
    let (client,task) = server(vec![discovery(),failed]);
    let error = match client.login("fixture",Secret::new("private-fixture".into())).await { Err(error) => error,Ok(_) => panic!("expected failure") };
    assert_eq!(error.retry_after(),Some(&RetryAfter::DelaySeconds(17)));
    assert!(matches!(&error,Error::Http { status:429,.. }));
    assert!(!format!("{error:?}").contains("private-body-sentinel"));
    assert!(!error.to_string().contains("Retry-After"));
    task.join().unwrap();
    let failed = Step { method:"GET",path:"/sxapi/v1/capabilities",response:"HTTP/1.1 503 Service Unavailable\r\nRetry-After: Wed, 21 Oct 2015 07:28:00 GMT\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".into(),headers:vec![] };
    let (client,task) = server(vec![failed]);
    let error = client.capabilities().await.unwrap_err();
    assert_eq!(error.retry_after(),Some(&RetryAfter::HttpDate(SystemTime::UNIX_EPOCH+Duration::from_secs(1_445_412_480))));
    assert!(matches!(&error,Error::Http { status:503,.. }));
    task.join().unwrap();
}

#[tokio::test]
async fn video_ticket_header_range_and_explicit_end() {
    let video = Step { method:"GET", path:"/sxapi/v1/playback/sessions/22222222-2222-4222-8222-222222222222/stream", response:"HTTP/1.1 206 Partial Content\r\nContent-Length: 4\r\nContent-Range: bytes 0-3/100\r\nETag: \"v1\"\r\nConnection: close\r\n\r\ndata".into(), headers:vec![(VIDEO_HEADER,"fixture-video-ticket".into()),("Range","bytes=0-3".into())] };
    let (client, task) = server(vec![discovery(),login_step(),discovery(),prepared(),video,end_step()]);
    let login = client.login("fixture", Secret::new("private-fixture".into())).await.unwrap();
    let mut playback = login.prepare(&playback_request()).await.unwrap();
    let range = playback.read_range(0,3,None).await.unwrap();
    assert_eq!(range.bytes,b"data");
    assert_eq!(range.total_length,100);
    playback.end().await.unwrap();
    assert!(matches!(playback.read_range(0,3,None).await,Err(Error::SessionEnded)));
    task.join().unwrap();
}

#[tokio::test]
async fn full_response_is_never_appended_to_range() {
    let video = Step { method:"GET", path:"/sxapi/v1/playback/sessions/22222222-2222-4222-8222-222222222222/stream", response:"HTTP/1.1 200 OK\r\nContent-Length: 4\r\nConnection: close\r\n\r\ndata".into(), headers:vec![] };
    let (client, task) = server(vec![discovery(),login_step(),discovery(),prepared(),video,end_step()]);
    let login = client.login("fixture",Secret::new("private-fixture".into())).await.unwrap();
    let mut playback = login.prepare(&playback_request()).await.unwrap();
    assert!(matches!(playback.read_range(0,3,Some("\"old\"")).await,Err(Error::RepresentationChanged)));
    playback.end().await.unwrap();
    task.join().unwrap();
}

#[tokio::test]
async fn wrong_content_range_is_rejected() {
    let video = Step { method:"GET", path:"/sxapi/v1/playback/sessions/22222222-2222-4222-8222-222222222222/stream", response:"HTTP/1.1 206 Partial Content\r\nContent-Length: 4\r\nContent-Range: bytes 4-7/100\r\nConnection: close\r\n\r\ndata".into(), headers:vec![] };
    let (client, task) = server(vec![discovery(),login_step(),discovery(),prepared(),video,end_step()]);
    let login = client.login("fixture",Secret::new("private-fixture".into())).await.unwrap();
    let mut playback = login.prepare(&playback_request()).await.unwrap();
    assert!(matches!(playback.read_range(0,3,None).await,Err(Error::RangeMismatch)));
    playback.end().await.unwrap();
    task.join().unwrap();
}

#[tokio::test]
async fn invalid_grant_is_closed_without_url_fallback() {
    let wrong = prepared_path("https://other.example/read", 100);
    let (client,task) = server(vec![discovery(),login_step(),discovery(),wrong,end_step()]);
    let login = client.login("fixture",Secret::new("private-fixture".into())).await.unwrap();
    assert!(matches!(login.prepare(&playback_request()).await,Err(Error::InvalidResponse)));
    task.join().unwrap();
}

#[tokio::test]
async fn private_video_uses_origin_and_header_only() {
    let video = Step { method:"GET",path:"/nimbus-play/v1/sxapi",response:"HTTP/1.1 206 Partial Content\r\nContent-Length: 4\r\nContent-Range: bytes 0-3/100\r\nConnection: close\r\n\r\ndata".into(),headers:vec![(VIDEO_HEADER,"fixture-video-ticket".into())] };
    let (client,task) = server(vec![discovery(),login_step(),discovery(),prepared_path("/nimbus-play/v1/sxapi",100),video,end_step()]);
    let login = client.login("fixture",Secret::new("private-fixture".into())).await.unwrap();
    let mut playback = login.prepare(&playback_request()).await.unwrap();
    assert_eq!(playback.read_range(0,3,None).await.unwrap().bytes,b"data");
    playback.end().await.unwrap();
    task.join().unwrap();
}

#[tokio::test]
async fn near_expiry_renews_before_new_range() {
    let renewal = json_step("POST","/sxapi/v1/playback/sessions/22222222-2222-4222-8222-222222222222/tickets",serde_json::json!({"stream_path":"/nimbus-play/v1/sxapi","ticket":"renewed-fixture","header_name":VIDEO_HEADER,"expires_at":(Utc::now()+chrono::Duration::seconds(80)).to_rfc3339()}));
    let video = Step { method:"GET",path:"/nimbus-play/v1/sxapi",response:"HTTP/1.1 206 Partial Content\r\nContent-Length: 4\r\nContent-Range: bytes 0-3/100\r\nConnection: close\r\n\r\ndata".into(),headers:vec![(VIDEO_HEADER,"renewed-fixture".into())] };
    let (client,task) = server(vec![discovery(),login_step(),discovery(),prepared_path("/nimbus-play/v1/sxapi",5),renewal,video,end_step()]);
    let seen = Arc::new(std::sync::Mutex::new(Vec::new()));
    let capture = seen.clone();
    let client = client.with_request_observer(move |request|capture.lock().unwrap().push(request.clone()));
    let login = client.login("fixture",Secret::new("private-fixture".into())).await.unwrap();
    let mut playback = login.prepare(&playback_request()).await.unwrap();
    playback.read_range(0,3,None).await.unwrap();
    playback.end().await.unwrap();
    assert!(seen.lock().unwrap().iter().all(|request|matches!(request.purpose,RequestPurpose::Interaction)));
    task.join().unwrap();
}

#[tokio::test]
async fn prefetch_purpose_survives_automatic_ticket_renewal() {
    let mut renewal = json_step("POST","/sxapi/v1/playback/sessions/22222222-2222-4222-8222-222222222222/tickets",serde_json::json!({"stream_path":"/nimbus-play/v1/sxapi","ticket":"renewed-fixture","header_name":VIDEO_HEADER,"expires_at":(Utc::now()+chrono::Duration::seconds(80)).to_rfc3339()}));
    renewal.headers.push(("Sec-Purpose","prefetch".into()));
    let video = Step { method:"GET",path:"/nimbus-play/v1/sxapi",response:"HTTP/1.1 206 Partial Content\r\nContent-Length: 4\r\nContent-Range: bytes 0-3/100\r\nConnection: close\r\n\r\ndata".into(),headers:vec![(VIDEO_HEADER,"renewed-fixture".into()),("Sec-Purpose","prefetch".into())] };
    let (client,task) = server(vec![discovery(),login_step(),discovery(),prepared_path("/nimbus-play/v1/sxapi",5),renewal,video,end_step()]);
    let seen = Arc::new(std::sync::Mutex::new(Vec::new()));
    let capture = seen.clone();
    let client = client.with_request_observer(move |request|capture.lock().unwrap().push(request.clone()));
    let login = client.login("fixture",Secret::new("private-fixture".into())).await.unwrap();
    let mut playback = login.prepare(&playback_request()).await.unwrap();
    playback.read_range_with_purpose(0,3,None,RequestPurpose::Prefetch).await.unwrap();
    playback.end().await.unwrap();
    let methods: Vec<_> = seen.lock().unwrap().iter().filter(|request|matches!(request.purpose,RequestPurpose::Prefetch)).map(|request|request.method.clone()).collect();
    assert_eq!(methods,vec!["POST","GET"]);
    task.join().unwrap();
}

#[tokio::test]
async fn subtitle_requires_original_bearer_and_dedicated_header() {
    let id = "a".repeat(64);
    let ticket_path = Box::leak(format!("/sxapi/v1/playback/sessions/22222222-2222-4222-8222-222222222222/subtitles/{id}/tickets").into_boxed_str());
    let grant = json_step("POST",ticket_path,serde_json::json!({"read_path":"/nimbus-play/v1/sxapi-subtitle","ticket":"subtitle-fixture","header_name":SUBTITLE_HEADER,"expires_at":(Utc::now()+chrono::Duration::seconds(80)).to_rfc3339()}));
    let text = Step { method:"GET",path:"/nimbus-play/v1/sxapi-subtitle",response:"HTTP/1.1 200 OK\r\nContent-Length: 4\r\nConnection: close\r\n\r\ntext".into(),headers:vec![(SUBTITLE_HEADER,"subtitle-fixture".into()),("Authorization",["Bearer stx_sx_","fixture"].concat())] };
    let (client,task) = server(vec![discovery(),login_step(),discovery(),prepared(),discovery(),grant,text,end_step()]);
    let login = client.login("fixture",Secret::new("private-fixture".into())).await.unwrap();
    let mut playback = login.prepare(&playback_request()).await.unwrap();
    assert_eq!(playback.subtitle(&id).await.unwrap(),b"text");
    playback.end().await.unwrap();
    task.join().unwrap();
}

#[tokio::test]
async fn oversized_json_is_rejected_from_headers() {
    let libraries = Step { method:"GET",path:"/sxapi/v1/libraries",response:format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",JSON_CAP+1),headers:vec![] };
    let (client,task) = server(vec![discovery(),login_step(),libraries]);
    let login = client.login("fixture",Secret::new("private-fixture".into())).await.unwrap();
    assert!(matches!(login.libraries().await,Err(Error::ResponseTooLarge)));
    task.join().unwrap();
}

#[test]
fn page_total_follows_the_optional_count_contract() {
    let skipped: Page<Media> = serde_json::from_value(serde_json::json!({"data": [], "total": null, "next_cursor": null})).unwrap();
    assert_eq!(skipped.total, None);
    let counted: Page<Media> = serde_json::from_value(serde_json::json!({"data": [], "total": 3, "next_cursor": "c"})).unwrap();
    assert_eq!(counted.total, Some(3));
    assert_eq!(counted.next_cursor.as_deref(), Some("c"));
}

#[test]
fn typed_queries_and_refresh_shapes_match_the_synchronized_contract() {
    let document: Value = serde_json::from_str(include_str!("../../../spec/sxapi-v1-client.openapi.json")).unwrap();
    assert_eq!(parameter(&document,"/media/removed","since")["required"],true);
    assert_eq!(parameter(&document,"/media/removed","limit")["schema"]["maximum"],5000);
    assert_eq!(parameter(&document,"/media/removed","cursor")["schema"]["maxLength"],4096);
    assert_eq!(parameter(&document,"/media/{id}/images/{kind}","language")["schema"]["maxLength"],35);
    let schemas = &document["components"]["schemas"];
    assert_eq!(schemas["Capabilities"]["properties"]["limits"]["properties"]["media_removed_page_size_max"]["const"],5000);
    assert_eq!(schemas["MediaRemovalHistoryUnavailable"]["properties"]["error"]["properties"]["code"]["const"],"removed_history_unavailable");
    assert_eq!(schemas["OwnCredentialRefresh"]["properties"]["refresh_token"]["pattern"],"^stx_sx(rt|ls)_[0-9a-f]{64}$");
    assert_eq!(schemas["SessionRefreshed"]["properties"]["refresh_token"]["pattern"],"^stx_sxls_[0-9a-f]{64}$");
    let answers = document["paths"]["/sessions/refresh"]["post"]["responses"]["200"]["content"]["application/json"]["schema"]["oneOf"].as_array().unwrap();
    assert_eq!(answers.len(),2);
    for feature in ["logo_languages","session_refresh","media_removed"] {
        assert_eq!(schemas["Capabilities"]["properties"]["features"]["properties"][feature]["const"],true);
    }
}

#[tokio::test]
async fn login_keeps_expiry_and_the_rotating_refresh_token() {
    let token = rotating('a');
    let login = json_step("POST","/sxapi/v1/sessions",serde_json::json!({"access_token":(["stx_sx_","fixture"].concat()),"token_type":"Bearer","user":{"id":"11111111-1111-4111-8111-111111111111","username":"fixture","library_ids":[]},"expires_at":"2026-11-27T00:00:00Z","refresh_token":token,"refresh_expires_at":"2026-12-26T00:00:00Z"}));
    let (client,task) = server(vec![discovery_with(&["session_refresh"]),login,discovery(),login_step()]);
    let login = client.login("fixture",Secret::new("private-fixture".into())).await.unwrap();
    assert_eq!(login.expires_at(),Some(instant("2026-11-27T00:00:00Z")));
    let renewal = login.renewal().unwrap();
    assert_eq!(renewal.refresh_token.expose_secret(),token.as_str());
    assert_eq!(renewal.refresh_expires_at,instant("2026-12-26T00:00:00Z"));
    assert!(!format!("{renewal:?}").contains(token.as_str()));
    // A server before 0.9.0-rc.27 sends neither the expiry nor a refresh token.
    let older = client.login("fixture",Secret::new("private-fixture".into())).await.unwrap();
    assert!(older.expires_at().is_none() && older.renewal().is_none());
    task.join().unwrap();
}

#[tokio::test]
async fn login_rejects_a_refresh_token_without_its_pair_or_shape() {
    let user = serde_json::json!({"id":"11111111-1111-4111-8111-111111111111","username":"fixture","library_ids":[]});
    let half = json_step("POST","/sxapi/v1/sessions",serde_json::json!({"access_token":(["stx_sx_","fixture"].concat()),"token_type":"Bearer","user":user,"expires_at":"2026-11-27T00:00:00Z","refresh_token":rotating('a'),"refresh_expires_at":null}));
    let personal = json_step("POST","/sxapi/v1/sessions",serde_json::json!({"access_token":(["stx_sx_","fixture"].concat()),"token_type":"Bearer","user":user,"expires_at":"2026-11-27T00:00:00Z","refresh_token":format!("stx_sxrt_{}","a".repeat(64)),"refresh_expires_at":"2026-12-26T00:00:00Z"}));
    let (client,task) = server(vec![discovery(),half,discovery(),personal]);
    for _ in 0..2 {
        assert!(matches!(client.login("fixture",Secret::new("private-fixture".into())).await,Err(Error::InvalidResponse)));
    }
    task.join().unwrap();
}

#[tokio::test]
async fn login_refresh_returns_the_next_token_before_the_new_login_is_used() {
    let next = rotating('b');
    let refreshed = json_step("POST","/sxapi/v1/sessions/refresh",serde_json::json!({"access_token":(["stx_sx_","renewed"].concat()),"token_type":"Bearer","expires_at":"2026-11-27T00:00:00Z","login_id":"66666666-6666-4666-8666-666666666666","refresh_token":next,"refresh_expires_at":"2026-12-26T00:00:00Z"}));
    let mut me = json_step("GET","/sxapi/v1/users/me",serde_json::json!({"id":"11111111-1111-4111-8111-111111111111","username":"fixture","library_ids":[]}));
    me.headers.push(("Authorization",["Bearer stx_sx_","renewed"].concat()));
    let (client,task) = server(vec![discovery_with(&["session_refresh"]),refreshed,me]);
    let seen = Arc::new(std::sync::Mutex::new(Vec::new()));
    let capture = seen.clone();
    let client = client.with_request_observer(move |request|capture.lock().unwrap().push(request.method.clone()));
    let refreshed = client.refresh(Secret::new(rotating('a'))).await.unwrap();
    // Nothing has used the new access token yet, so the caller can store the next token first.
    assert_eq!(*seen.lock().unwrap(),vec!["GET","POST"]);
    assert_eq!(refreshed.renewal().unwrap().refresh_token.expose_secret(),next.as_str());
    assert_eq!(refreshed.expires_at(),Some(instant("2026-11-27T00:00:00Z")));
    let login = refreshed.activate().await.unwrap();
    assert_eq!(login.user().username,"fixture");
    assert_eq!(login.renewal().unwrap().refresh_expires_at,instant("2026-12-26T00:00:00Z"));
    assert_eq!(seen.lock().unwrap().len(),3);
    task.join().unwrap();
}

#[tokio::test]
async fn personal_secret_refresh_is_not_rotated() {
    let refreshed = json_step("POST","/sxapi/v1/sessions/refresh",serde_json::json!({"access_token":(["stx_sx_","child"].concat()),"token_type":"Bearer","expires_at":"2026-11-27T00:00:00Z","login_id":"66666666-6666-4666-8666-666666666666","credential_id":"77777777-7777-4777-8777-777777777777"}));
    let me = json_step("GET","/sxapi/v1/users/me",serde_json::json!({"id":"11111111-1111-4111-8111-111111111111","username":"fixture","library_ids":[]}));
    let (client,task) = server(vec![discovery(),refreshed,me]);
    let refreshed = client.refresh(Secret::new(format!("stx_sxrt_{}","c".repeat(64)))).await.unwrap();
    assert!(refreshed.renewal().is_none());
    let login = refreshed.activate().await.unwrap();
    assert!(login.renewal().is_none());
    assert_eq!(login.expires_at(),Some(instant("2026-11-27T00:00:00Z")));
    task.join().unwrap();
}

#[tokio::test]
async fn refresh_checks_the_secret_kind_feature_and_answer_shape() {
    // Malformed secrets never leave the client.
    let (client,task) = server(vec![]);
    for value in ["stx_sxls_short".to_owned(),format!("stx_sx_{}","a".repeat(64)),format!("stx_sxls_{}","A".repeat(64))] {
        assert!(matches!(client.refresh(Secret::new(value)).await,Err(Error::InvalidCredential)));
    }
    task.join().unwrap();
    // A login refresh token needs the discovered feature.
    let (client,task) = server(vec![discovery()]);
    assert!(matches!(client.refresh(Secret::new(rotating('a'))).await,Err(Error::UnsupportedFeature)));
    task.join().unwrap();
    // The answer must be the shape of the secret that was sent.
    let personal_answer = json_step("POST","/sxapi/v1/sessions/refresh",serde_json::json!({"access_token":(["stx_sx_","child"].concat()),"token_type":"Bearer","expires_at":"2026-11-27T00:00:00Z","login_id":"66666666-6666-4666-8666-666666666666","credential_id":"77777777-7777-4777-8777-777777777777"}));
    let login_answer = json_step("POST","/sxapi/v1/sessions/refresh",serde_json::json!({"access_token":(["stx_sx_","renewed"].concat()),"token_type":"Bearer","expires_at":"2026-11-27T00:00:00Z","login_id":"66666666-6666-4666-8666-666666666666","refresh_token":rotating('b'),"refresh_expires_at":"2026-12-26T00:00:00Z"}));
    let (client,task) = server(vec![discovery_with(&["session_refresh"]),personal_answer,discovery(),login_answer]);
    assert!(matches!(client.refresh(Secret::new(rotating('a'))).await,Err(Error::InvalidResponse)));
    assert!(matches!(client.refresh(Secret::new(format!("stx_sxrt_{}","c".repeat(64)))).await,Err(Error::InvalidResponse)));
    task.join().unwrap();
}

#[tokio::test]
async fn removed_media_pages_from_an_inclusive_instant() {
    let since = instant("2026-11-26T01:02:03.123456789Z");
    let page = json_step("GET","/sxapi/v1/media/removed?since=2026-11-26T01%3A02%3A03.123456Z&limit=2&cursor=opaque-1",serde_json::json!({"data":[{"id":"33333333-3333-4333-8333-333333333333","removed_at":"2026-11-26T01:05:00Z","reason":"deleted"}],"next_cursor":null,"tracked_since":"2026-11-01T00:00:00Z"}));
    let (client,task) = server(vec![discovery_with(&["media_removed"]),login_step(),page]);
    let login = client.login("fixture",Secret::new("private-fixture".into())).await.unwrap();
    let page = login.removed_media(since,Some("opaque-1"),2).await.unwrap();
    assert_eq!(page.data.len(),1);
    assert_eq!(page.data[0].id,Uuid::parse_str("33333333-3333-4333-8333-333333333333").unwrap());
    assert_eq!(page.data[0].reason,"deleted");
    assert_eq!(page.data[0].removed_at,instant("2026-11-26T01:05:00Z"));
    assert!(page.next_cursor.is_none());
    assert_eq!(page.tracked_since,instant("2026-11-01T00:00:00Z"));
    // Bounds are checked before any request.
    for limit in [0,5001] { assert!(matches!(login.removed_media(since,None,limit).await,Err(Error::InvalidInput))); }
    assert!(matches!(login.removed_media(since,Some(""),10).await,Err(Error::InvalidInput)));
    task.join().unwrap();
}

#[tokio::test]
async fn removed_history_gap_is_a_typed_error() {
    let gone = |body: &str| Step { method:"GET",path:"/sxapi/v1/media/removed?since=2026-11-01T00%3A00%3A00.000000Z&limit=1000",response:format!("HTTP/1.1 410 Gone\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()),headers:vec![] };
    let tracked = gone(r#"{"error":{"code":"removed_history_unavailable","message":"private-body-sentinel"},"tracked_since":"2026-11-25T00:00:00Z"}"#);
    let unswept = gone(r#"{"error":{"code":"removed_history_unavailable","message":"fixture"},"tracked_since":null}"#);
    let other = gone(r#"{"error":{"code":"fixture_gone","message":"fixture"}}"#);
    let (client,task) = server(vec![discovery_with(&["media_removed"]),login_step(),tracked,unswept,other]);
    let login = client.login("fixture",Secret::new("private-fixture".into())).await.unwrap();
    let since = instant("2026-11-01T00:00:00Z");
    let error = login.removed_media(since,None,1000).await.unwrap_err();
    assert_eq!(error,Error::RemovedHistoryUnavailable { tracked_since:Some(instant("2026-11-25T00:00:00Z")) });
    assert!(!format!("{error:?}").contains("private-body-sentinel"));
    assert_eq!(login.removed_media(since,None,1000).await.unwrap_err(),Error::RemovedHistoryUnavailable { tracked_since:None });
    assert!(matches!(login.removed_media(since,None,1000).await,Err(Error::Http { status:410,code:Some(code),.. }) if code == "fixture_gone"));
    task.join().unwrap();
    // Without the discovered feature nothing is sent.
    let (client,task) = server(vec![discovery(),login_step()]);
    let login = client.login("fixture",Secret::new("private-fixture".into())).await.unwrap();
    assert!(matches!(login.removed_media(since,None,1000).await,Err(Error::UnsupportedFeature)));
    task.join().unwrap();
}

#[test]
fn media_logos_are_typed_and_default_to_empty() {
    let media: Media = serde_json::from_value(serde_json::json!({"id":"33333333-3333-4333-8333-333333333333","name":"Fixture","media_type":"movie","has_logo":true,"logos":[{"lang":"zh-TW","w":500,"h":200},{"lang":"xx","w":null,"h":null}]})).unwrap();
    assert_eq!(media.logos.len(),2);
    assert_eq!(media.logos[0].lang,"zh-TW");
    assert_eq!((media.logos[0].w,media.logos[0].h,media.logos[1].w),(Some(500),Some(200),None));
    assert!(!media.extra.contains_key("logos"));
    assert_eq!(media.extra["has_logo"],true);
    let older: Media = serde_json::from_value(serde_json::json!({"id":"33333333-3333-4333-8333-333333333333","name":"Fixture","media_type":null})).unwrap();
    assert!(older.logos.is_empty());
}

#[tokio::test]
async fn logo_language_is_sent_only_with_the_discovered_feature() {
    let bearer = ["Bearer stx_sx_","fixture"].concat();
    let image = |path: &'static str| Step { method:"GET",path,response:"HTTP/1.1 200 OK\r\nContent-Type: image/png\r\nContent-Length: 4\r\nConnection: close\r\n\r\nlogo".into(),headers:vec![("Authorization",bearer.clone())] };
    let chosen = image("/sxapi/v1/media/33333333-3333-4333-8333-333333333333/images/logo?language=zh-TW");
    let fallback = image("/sxapi/v1/media/33333333-3333-4333-8333-333333333333/images/logo");
    let (client,task) = server(vec![discovery_with(&["images","logo_languages"]),login_step(),chosen,fallback]);
    let login = client.login("fixture",Secret::new("private-fixture".into())).await.unwrap();
    let media = Uuid::parse_str("33333333-3333-4333-8333-333333333333").unwrap();
    assert_eq!(login.logo(media,Some("zh-TW"),1024).await.unwrap(),b"logo");
    assert_eq!(login.logo(media,None,1024).await.unwrap(),b"logo");
    for bad in [String::new(),"zh TW".into(),"zh_TW".into(),"a".repeat(36)] {
        assert!(matches!(login.logo(media,Some(bad.as_str()),1024).await,Err(Error::InvalidInput)));
    }
    task.join().unwrap();
    let (client,task) = server(vec![discovery_with(&["images"]),login_step()]);
    let login = client.login("fixture",Secret::new("private-fixture".into())).await.unwrap();
    assert!(matches!(login.logo(media,Some("en"),1024).await,Err(Error::UnsupportedFeature)));
    task.join().unwrap();
}
