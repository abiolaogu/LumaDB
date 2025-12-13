use bson::{Document, doc, DateTime};
use chrono::Utc;

pub fn handle_hello() -> Document {
    // Current time
    let local_time = DateTime::now();
    
    doc! {
        "ismaster": true,
        "maxBsonObjectSize": 16777216,
        "maxMessageSizeBytes": 48000000,
        "maxWriteBatchSize": 100000,
        "localTime": local_time,
        "logicalSessionTimeoutMinutes": 30,
        "connectionId": 1, // Placeholder
        "minWireVersion": 0,
        "maxWireVersion": 17, // MongoDB 6.0
        "readOnly": false,
        "ok": 1.0
    }
}

pub fn handle_build_info() -> Document {
    let now = DateTime::now();
    doc! {
        "version": "6.0.0",
        "gitVersion": "unknown",
        "modules": [],
        "allocator": "system",
        "javascriptEngine": "mozjs",
        "sysInfo": "deprecated",
        "versionArray": [6, 0, 0, 0],
        "openssl": {
            "running": "OpenSSL 1.1.1",
            "compiled": "OpenSSL 1.1.1"
        },
        "buildEnvironment": {
            "distmod": "ubuntu2004",
            "distarch": "x86_64",
            "cc": "/opt/go/bin/gcc",
            "ccflags": "-Werror -static-libgcc",
            "cxx": "/opt/go/bin/g++",
            "cxxflags": "-Werror -static-libgcc",
            "linkflags": "-Wl,-z,now -rdynamic",
            "target_arch": "x86_64",
            "target_os": "linux"
        },
        "bits": 64,
        "debug": false,
        "maxBsonObjectSize": 16777216,
        "storageEngines": [ "wiredTiger" ], // Pretend to be standard
        "ok": 1.0
    }
}
