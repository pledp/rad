mod common;

use tokio::net::{TcpListener, TcpStream};
use tracing::info;
use tracing_subscriber::fmt;

use eradic::ul::{
    connection::{StateTransition, UpperLayerConnectionState},
    event::{Event, ServiceProviderToServiceUser, ServiceUserToServiceProvider},
};
use eradic_ul_tokio::requestor_handle_connection;

use common::service_user::create_server_one_connection;
use common::util::*;

#[tokio::test]
async fn test_association_requestor_sta1_to_sta6_ok() {
    let server = TcpListener::bind("127.0.0.1:11104").await.unwrap();
    create_server_one_connection(server, |indication, tx| async move {
        if let ServiceProviderToServiceUser::AssociateIndicationPrimitive(ind) = indication {
            let response = accept_all_response(ind);

            tx.send(ServiceUserToServiceProvider::AssociateResponsePrimitive(response))
                .await
                .ok();
        }
    })
    .await;

    let stream = TcpStream::connect("127.0.0.1:11104").await.unwrap();

    let request = default_associate_request(&stream);
    let accepted_response = accept_all_response(request.clone());

    let expected_event_and_state = vec![
        StateTransition {
            event: None,
            state: UpperLayerConnectionState::Idle,
        },
        StateTransition {
            event: Some(Event::AssociateRequestPrimitive(request.clone())),
            state: UpperLayerConnectionState::WaitingForOpenConnection,
        },
        StateTransition {
            event: Some(Event::TransportConnectionConfirmation {
                called_address: stream.local_addr().unwrap().ip(),
                called_port: stream.local_addr().unwrap().port(),
                calling_address: stream.peer_addr().unwrap().ip(),
                calling_port: stream.peer_addr().unwrap().port(),
            }),
            state: UpperLayerConnectionState::WaitingForAcRjPdu,
        },
        StateTransition {
            event: Some(Event::AssociateAcceptPdu(accepted_response.try_into().unwrap())),
            state: UpperLayerConnectionState::DataTransfer,
        }
    ];

    let socket_addr = stream.peer_addr().unwrap();
    let mut handle = requestor_handle_connection(stream, socket_addr, request).unwrap();

    let mut state_rx = handle.state_mpsc;

    let state_task = tokio::spawn(async move {
        let mut states = vec![];
        while let Some(transition) = state_rx.recv().await {
            info!("{:?}", transition);
            states.push(transition);
        }
        states
    });

    while let Some(ind) = handle.scp_to_scu_rx.recv().await {
        handle.task.abort();
    }

    let observed_transistions = state_task.await.unwrap();

    assert_eq!(observed_transistions, expected_event_and_state);
}
