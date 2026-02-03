/* eslint-disable no-console */
import { Device } from 'mediasoup-client';
import { type DataConsumer, type MediaKind, type RtpCapabilities, type RtpParameters } from 'mediasoup-client/types';
import { type DtlsParameters, type TransportOptions, type Transport } from 'mediasoup-client/types';
import { type DataConsumerOptions } from 'mediasoup-client/types';

type Brand<K, T> = K & { __brand: T };

type ConsumerId = Brand<string, 'ConsumerId'>;
type ProducerId = Brand<string, 'ProducerId'>;
type DataProducerId = Brand<string, 'DataProducerId'>;
type DataConsumerId = Brand<string, 'DataDataConsumerId'>;
interface ServerInit {
  action: 'Init';
  consumerTransportOptions: TransportOptions;
  producerTransportOptions: TransportOptions;
  routerRtpCapabilities: RtpCapabilities;
  dataProducerId: DataProducerId;
}

interface ServerConnectedProducerTransport {
  action: 'ConnectedProducerTransport';
}

interface ServerProduced {
  action: 'Produced';
  id: ProducerId;
}

interface ServerConnectedConsumerTransport {
  action: 'ConnectedConsumerTransport';
}

interface ServerConsumed {
  action: 'Consumed';
  id: ConsumerId;
  kind: MediaKind;
  rtpParameters: RtpParameters;
}

interface ServerDataConsumed {
  action: 'DataConsumed';
  id: DataConsumerId;
  dataProducerId: DataProducerId;
}
type ServerMessage =
  ServerInit |
  ServerConnectedProducerTransport |
  ServerProduced |
  ServerConnectedConsumerTransport |
  ServerConsumed | ServerDataConsumed;

interface ClientInit {
  action: 'Init';
  rtpCapabilities: RtpCapabilities;
}

interface ClientConnectProducerTransport {
  action: 'ConnectProducerTransport';
  dtlsParameters: DtlsParameters;
}

interface ClientConnectConsumerTransport {
  action: 'ConnectConsumerTransport';
  dtlsParameters: DtlsParameters;
}

interface ClientProduce {
  action: 'Produce';
  kind: MediaKind;
  rtpParameters: RtpParameters;
}

interface ClientConsume {
  action: 'Consume';
  producerId: ProducerId;
}

interface ClientConsumerResume {
  action: 'ConsumerResume';
  id: ConsumerId;
}
interface ClientDataConsume {
  action: 'DataConsume';
  dataProducerId: DataProducerId;
}

interface ClientDataConsumerResume {
  action: 'DataConsumerResume';
  id: DataConsumerId;
}

type ClientMessage =
  ClientInit |
  ClientConnectProducerTransport |
  ClientProduce |
  ClientConnectConsumerTransport |
  ClientConsume |
  ClientConsumerResume | ClientDataConsume | ClientDataConsumerResume;

async function init() {

  const ws = new WebSocket('ws://10.6.5.218:5177/ws');
  function send(message: ClientMessage) {
    ws.send(JSON.stringify(message));
  }

  const device = new Device();
  let producerTransport: Transport | undefined;
  let consumerTransport: Transport | undefined;
  let dataConsumer: DataConsumer;
  {
    const waitingForResponse: Map<ServerMessage['action'], Function> = new Map();

    ws.onmessage = async (message) => {
      const decodedMessage: ServerMessage = JSON.parse(message.data);

      switch (decodedMessage.action) {
        case 'Init': {
          // It is expected that server will send initialization message right after
          // WebSocket connection is established
          await device.load({
            routerRtpCapabilities: decodedMessage.routerRtpCapabilities
          });
          console.log(decodedMessage.dataProducerId);

          // Send client-side initialization message back right away
          send({
            action: 'Init',
            rtpCapabilities: device.rtpCapabilities
          });


          consumerTransport = device.createRecvTransport(
            decodedMessage.consumerTransportOptions
          );

          consumerTransport
            .on('connect', ({ dtlsParameters }, success) => {
              // Send request to establish consumer transport connection
              send({
                action: 'ConnectConsumerTransport',
                dtlsParameters
              });
              // And wait for confirmation, but, obviously, no error handling,
              // which you should definitely have in real-world applications
              waitingForResponse.set('ConnectedConsumerTransport', () => {
                success();
                console.log('Consumer transport connected');
              });
            });

          // For simplicity of this example producers were stored in an array
          // and are now all consumed one at a time
          // for (const producer of producers) {
          await new Promise((resolve) => {
            // Send request to consume producer
            send({
              action: 'DataConsume',
              dataProducerId: decodedMessage.dataProducerId
            });
            // And wait for confirmation, but, obviously, no error handling,
            // which you should definitely have in real-world applications
            waitingForResponse.set('DataConsumed', async (consumerOptions: DataConsumerOptions) => {
              // Once confirmation is received, corresponding consumer
              // can be created client-side
              console.log(consumerOptions)
              dataConsumer = await (consumerTransport as Transport).consumeData(
                consumerOptions
              );

              dataConsumer.on("message", (a) => {
                console.log(a)
              })

              console.log(`${dataConsumer.dataProducerId} consumer created:`, dataConsumer);
              dataConsumer.on("open", () => {
                console.log("open")
              })

              send({
                action: 'DataConsumerResume',
                id: dataConsumer.id as DataConsumerId
              });


              resolve(undefined);
            });
          });

          break;
        }
        default: {
          // All messages other than initialization go here and are assumed
          // to be notifications that correspond to previously sent requests
          const callback = waitingForResponse.get(decodedMessage.action);

          if (callback) {
            waitingForResponse.delete(decodedMessage.action);
            callback(decodedMessage);
          }
          else {
            console.error('Received unexpected message', decodedMessage);
          }
        }
      }
    };
  }
  ws.onerror = console.error;
}

init();
