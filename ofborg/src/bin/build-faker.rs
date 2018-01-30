extern crate ofborg;
extern crate amqp;
extern crate env_logger;

extern crate hyper;
extern crate hubcaps;
extern crate hyper_native_tls;


use std::env;

use amqp::Session;

use ofborg::config;
use ofborg::worker;
use ofborg::notifyworker;
use ofborg::notifyworker::NotificationReceiver;
use ofborg::commentparser;
use ofborg::message::buildjob;

use ofborg::message::{Pr, Repo};

fn main() {
    let cfg = config::load(env::args().nth(1).unwrap().as_ref());
    ofborg::setup_log();

    println!("Hello, world!");


    let mut session = Session::open_url(&cfg.rabbitmq.as_uri()).unwrap();
    println!("Connected to rabbitmq");


    let mut channel = session.open_channel(1).unwrap();

    let repo_msg = Repo {
        clone_url: "https://github.com/nixos/nixpkgs.git".to_owned(),
        full_name: "NixOS/nixpkgs".to_owned(),
        owner: "NixOS".to_owned(),
        name: "nixpkgs".to_owned(),
    };

    let pr_msg = Pr {
        number: 34402,
        head_sha: "90164e40d384613dc62a419a21b86358b53ecfd8".to_owned(),
        target_branch: Some("master".to_owned()),
    };

    let logbackrk = "NixOS/nixpkgs.34402".to_owned();

    let msg = buildjob::BuildJob {
        repo: repo_msg.clone(),
        pr: pr_msg.clone(),
        subset: Some(commentparser::Subset::NixOS),
        attrs: vec!["tests.openssh.x86_64-linux".to_owned()],
        logs: Some((Some("logs".to_owned()), Some(logbackrk.to_lowercase()))),
        statusreport: Some((None, Some("scratch".to_owned()))),
    };

    {
        let mut recv = notifyworker::ChannelNotificationReceiver::new(&mut channel, 0);

        for _i in 1..2 {
            recv.tell(worker::publish_serde_action(
                None,
                Some("build-inputs-aarch64-linux".to_owned()),
                &msg,
            ));
        }
    }

    channel.close(200, "Bye").unwrap();
    println!("Closed the channel");
    session.close(200, "Good Bye");
    println!("Closed the session... EOF");
}
