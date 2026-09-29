use enumset::EnumSetType;

/// Device roles determaines what type of behavors will by realized by the
/// node
#[derive(EnumSetType, Debug)]
pub enum DeviceRole {
  /// Mandatory role for any node. Determines behavior of receiving, processing 
  /// and retranslating of network packets.
  DEFAULT,

  /// Optional role. Node will be regularaly send hello packets through the network.
  /// Hello packets contains information about the node like name, applied 
  /// roles, neghboard and etc. This inforamation recived by another nodes allows
  /// lookup this node even if it does not send any valubal data through the network 
  /// and creates local model of network structure (in NodeRegistry) which may by
  /// used by client code for node descovery or debug needs. Inverval fo beaconing
  /// specified by configuraion parameter 'beacon_interval'. Also that mode assumes
  /// that node has specified name throught 'name' param.
  BEACON,

  /// Optional role. Node performs as gateway to another notwork. In this mode it 
  /// can not by used for direct send and receice messages. All interation with
  /// network perfomed through Gateway adapter provided by the client code. You must 
  /// implement trait Gateway and provide it in 'gateway' configuration parameter.
  /// This implementations must realise some features for that type communacation. 
  /// Such as for example - receiver and sender of mqtt provider. When data goes to 
  /// the gateway impl from the network, it must by converted to mqtt message and 
  /// than send to specified queue (determaied by source address for example). And
  /// opposite - when data goes from mqtt, adapter must determine dest node adress in
  /// the network and pass data to the to the net.
  GATEWAY
}