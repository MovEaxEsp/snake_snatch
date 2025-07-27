
use serde::{Serialize, Deserialize};
use serde::de::{DeserializeOwned};
use wasm_bindgen::prelude::*;

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

#[wasm_bindgen(module = "https://esm.sh/peerjs@1.5.5?bundle-deps")]
extern "C" {
    type Peer;
    
    #[wasm_bindgen(constructor)]
    fn new_with_str(endpoing: &str) -> Peer;

    #[wasm_bindgen(constructor)]
    fn new() -> Peer;
    
    #[wasm_bindgen(method)]
    fn destroy(peer: &Peer);
    
    #[wasm_bindgen(method, js_name="on")]
    fn on_cb(peer: &Peer, event: &str, cb: &JsValue);
    
    #[wasm_bindgen(method)]
    fn connect(peer: &Peer, id: &str) -> JsValue;
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = DataConnection)]
    type DataConnection;
    #[wasm_bindgen(method, js_name="on")]
    fn on_cb(dc: &DataConnection, event: &str, cb: &JsValue);

    #[wasm_bindgen(method)]
    fn send(dc: &DataConnection, msg: &JsValue);
    
    #[wasm_bindgen(method)]
    fn close(dc: &DataConnection);
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct NetData<MSG> 
where MSG: fmt::Debug + Serialize
{
    pub stream_id: i32,
    pub msg: MSG,
}

#[derive(Serialize, Deserialize, Clone)]
pub enum NetReqType<MSG>
where  MSG: fmt::Debug + Serialize
{
    Listen(String),
    Connect(String),
    Close,
    Send(NetData<MSG>),
}

#[derive(Serialize, Deserialize, Clone)]
pub struct NetReq<MSG>
where MSG: fmt::Debug + Serialize
{
    pub correlator: i32,
    pub req: NetReqType<MSG>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum NetUpdate<MSG>
where MSG: fmt::Debug + Serialize
{
    ListenFail,
    ConnectFail,
    NewPeer(i32),
    Data(NetData<MSG>), // stream_id + msg
    Closed,
}

#[derive(Clone, Copy, PartialEq)]
pub struct NetworkHandle(i32);

impl NetworkHandle {
    pub fn invalid() -> NetworkHandle {
        NetworkHandle { 0: 0 }
    }
    
    pub fn from_correlator(corr: i32) -> NetworkHandle {
        NetworkHandle { 0: corr }
    }
}

impl fmt::Display for NetworkHandle {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub struct PeerInfo<MSG>
where MSG: fmt::Debug + Serialize
{
    _is_listen: bool,
    _is_closed: bool,
    _peer: Option<Peer>,
    _peer_open_closure: Option<Closure::<dyn FnMut(String)>>,
    _peer_connection_closure: Option<Closure::<dyn FnMut(JsValue)>>,
    _peer_error_closure: Option<Closure::<dyn FnMut(JsValue)>>,
    _dc: Option<DataConnection>,
    _dc_open_closure: Option<Closure::<dyn FnMut()>>,
    _dc_data_closure: Option<Closure::<dyn FnMut(JsValue)>>,
    _dc_close_closure: Option<Closure::<dyn FnMut()>>,
    _dc_error_closure: Option<Closure::<dyn FnMut(JsValue)>>,
    received_msgs: HashMap<i32, Vec<NetUpdate<MSG>>>,
    next_stream_id: i32,
}

impl<MSG> PeerInfo<MSG>
where MSG: fmt::Debug + Serialize
{
    fn new(is_listen: bool) -> Self {
        Self {
            _is_listen: is_listen,
            _is_closed: false,
            _peer: None,
            _peer_open_closure: None,
            _peer_connection_closure: None,
            _peer_error_closure: None,
            _dc: None,
            _dc_open_closure: None,
            _dc_data_closure: None,
            _dc_close_closure: None,
            _dc_error_closure: None,
            received_msgs: HashMap::new(),
            next_stream_id: if is_listen {1} else {2},
        }
    }
    
    fn receive(&mut self, stream_id: i32, upd: NetUpdate<MSG>) {
        self.received_msgs.entry(stream_id).or_default().push(upd);
    }
}

struct NetworkManagerImp<MSG>
where MSG: fmt::Debug + Serialize
{
    handle_map: HashMap<i32, PeerInfo<MSG>>,
    next_handle: i32,
}

impl <MSG> NetworkManagerImp<MSG>
where MSG: DeserializeOwned + fmt::Debug + Serialize + 'static
{
    fn register_open_closure(imp_rc: Rc<RefCell<Self>>, dc: &DataConnection, src_handle: i32, dc_handle: i32)
    -> Closure::<dyn FnMut()>
    {
        let closure = Closure::<dyn FnMut()>::new(move || {
            log(&format!("DC::on(open) src_id: {}, id: {}", &src_handle, &dc_handle));
            let mut imp = imp_rc.borrow_mut();
            match imp.handle_map.get_mut(&src_handle) {
                Some(peer) => {
                    peer.receive(0, NetUpdate::NewPeer(dc_handle));
                }
                None => {
                    log(&format!("Got 'open' for unknown handle: {}", &src_handle));
                }
            }
        });
        dc.on_cb("open", closure.as_ref().unchecked_ref());
        closure
    }

    fn register_data_closure(imp_rc: Rc<RefCell<Self>>, dc: &DataConnection, dc_handle: i32)
    -> Closure::<dyn FnMut(JsValue)>
    {
        let closure = Closure::<dyn FnMut(JsValue)>::new(move |data| {
            match serde_wasm_bindgen::from_value::<NetData<MSG>>(data) {
                Ok(msg) => {
                    log(&format!("Net(data) handle: {}, data:{:?}", &dc_handle, &msg));
                    let mut imp = imp_rc.borrow_mut();
                    match imp.handle_map.get_mut(&dc_handle) {
                        Some(closure_peer) => {
                            closure_peer.receive(msg.stream_id, NetUpdate::Data(msg));
                        }
                        None => {
                            log(&format!("Got update for unknown handle: {}", &dc_handle));
                        }
                    }
                }
                Err(e) => {
                    log(&format!("Failed parsing msg '{}' for handle: {}", e, &dc_handle));
                }
            }
        });
        dc.on_cb("data", closure.as_ref().unchecked_ref());
        closure
    }

    fn register_close_closure(_imp_rc: Rc<RefCell<Self>>, dc: &DataConnection, dc_handle: i32)
    -> Closure::<dyn FnMut()>
    {
        let closure = Closure::<dyn FnMut()>::new(move || {
            log(&format!("DC::on(close) id: {}", &dc_handle));
            // TODO handle
        });
        dc.on_cb("close", closure.as_ref().unchecked_ref());
        closure
    }

    fn register_error_closure(_imp_rc: Rc<RefCell<Self>>, dc: &DataConnection, dc_handle: i32)
    -> Closure::<dyn FnMut(JsValue)>
    {
        let closure = Closure::<dyn FnMut(JsValue)>::new(move |err | {
            log(&format!("DC::on(error) id: {}, error: {:?}", &dc_handle, &err));
            // TODO handle
        });
        dc.on_cb("error", closure.as_ref().unchecked_ref());
        closure
    }

    fn handle_new_connection(imp_rc: Rc<RefCell<Self>>, peer: &mut PeerInfo<MSG>, dc_val:JsValue, src_handle: i32, dc_handle: i32) {
        let dc = dc_val.unchecked_into::<DataConnection>();

        peer._dc = Some(dc.clone().unchecked_into::<DataConnection>());
        peer._dc_open_closure = Some(Self::register_open_closure(imp_rc.clone(), &dc, src_handle, dc_handle));
        peer._dc_close_closure = Some(Self::register_close_closure(imp_rc.clone(), &dc, dc_handle));
        peer._dc_data_closure = Some(Self::register_data_closure(imp_rc.clone(), &dc, dc_handle));
        peer._dc_error_closure = Some(Self::register_error_closure(imp_rc.clone(), &dc, dc_handle));
    }
}

pub struct NetworkManager<MSG>
where MSG: fmt::Debug + Serialize
{
    imp: Rc<RefCell<NetworkManagerImp<MSG>>>,
}

impl<MSG> NetworkManager<MSG>
where MSG: Serialize + DeserializeOwned + fmt::Debug + 'static {
    pub fn new() -> Self {
        Self {
            imp: Rc::new(RefCell::new(NetworkManagerImp::<MSG> {
                handle_map: HashMap::new(),
                next_handle: 1,
            })),
        }
    }

    pub fn connect(&self, address: &str) -> NetworkHandle {
        let imp = &mut *self.imp.borrow_mut();

        let handle = imp.next_handle;
        imp.next_handle += 1;

        let new_peer = Peer::new();

        let imp_ref = self.imp.clone();
        let local_address: String = address.into();
        let open_closure = Closure::<dyn FnMut(String)>::new(move |id: String| {
            log(&format!("Net(Peer::open) handle: {}, address: {}, id: {}", &handle, &local_address, &id));
            
            let open_imp = &mut *imp_ref.borrow_mut();
            match open_imp.handle_map.get_mut(&handle) {
                Some(info) => {
                    let dc = info._peer.as_ref().unwrap().connect(&local_address);
                    NetworkManagerImp::<MSG>::handle_new_connection(imp_ref.clone(), info, dc, handle, handle);
                },
                None => {
                    log(&format!("Net(Peer::open) handle:{} already closed", &handle));                    
                }
            }
        });
        new_peer.on_cb("open", open_closure.as_ref().unchecked_ref());
        
        let error_closure = Closure::<dyn FnMut(JsValue)>::new(move |err| {
            log(&format!("Net(Peer::error) handle: {}, err: {:?}", &handle, &err));
            // TODO handle
        });
        new_peer.on_cb("error", error_closure.as_ref().unchecked_ref());

        let mut new_info = PeerInfo::<MSG>::new(false);
        new_info._peer = Some(new_peer);
        new_info._peer_open_closure = Some(open_closure);
        new_info._peer_error_closure = Some(error_closure);

        imp.handle_map.insert(handle, new_info);

        NetworkHandle { 0: handle }
    }
    
    pub fn listen(&self, address: &str) -> NetworkHandle {
        let imp = &mut *self.imp.borrow_mut();

        let handle = imp.next_handle;
        imp.next_handle += 1;

        let new_peer = Peer::new_with_str(address);

        let open_closure = Closure::<dyn FnMut(String)>::new(move |id: String| {
            log(&format!("Net(Peer::open) handle: {}, id: {}", &handle, &id));
        });
        new_peer.on_cb("open", open_closure.as_ref().unchecked_ref());

        let imp_ref = self.imp.clone();
        let connection_closure = Closure::<dyn FnMut(JsValue)>::new(move |dc: JsValue| {
            let inner_imp = &mut *imp_ref.borrow_mut();
            let dc_handle = inner_imp.next_handle;
            inner_imp.next_handle += 1;

            log(&format!("Net(Peer::connection) handle: {}, new_handle: {}", &handle, &dc_handle));
            
            let mut conn_peer = PeerInfo::<MSG>::new(true);
            
            NetworkManagerImp::<MSG>::handle_new_connection(imp_ref.clone(), &mut conn_peer, dc, handle, dc_handle);
            inner_imp.handle_map.insert(dc_handle, conn_peer);
        });
        new_peer.on_cb("connection", connection_closure.as_ref().unchecked_ref());

        let error_closure = Closure::<dyn FnMut(JsValue)>::new(move |err| {
            log(&format!("Net(Peer::error) handle: {}, err: {:?}", &handle, &err));
            // TODO handle
        });
        new_peer.on_cb("error", error_closure.as_ref().unchecked_ref());

        let mut new_info = PeerInfo::<MSG>::new(true);
        new_info._peer = Some(new_peer);
        new_info._peer_open_closure = Some(open_closure);
        new_info._peer_connection_closure = Some(connection_closure);
        new_info._peer_error_closure = Some(error_closure);

        imp.handle_map.insert(handle, new_info);

        NetworkHandle { 0: handle }
    }
    
    /// Cancel the operation/close the connection associted with the specified 'handle'
    pub fn close(&mut self, NetworkHandle(handle): NetworkHandle) {
        let imp = &mut *self.imp.borrow_mut();

        match imp.handle_map.remove(&handle) {
            Some(info) => {
                if let Some(dc) = info._dc {
                    dc.close();
                }
                if let Some(peer) = info._peer {
                    peer.destroy()
                }
                log(&format!("Net(close) handle:{}", &handle));
            }
            None => {
                log(&format!("Net(close) handle:{} already closed", &handle));
            }
        }
    }
    
    /// Send the specified 'msg' to the peer associated with the specified 'handle'.
    pub fn send(&mut self, NetworkHandle(handle): &NetworkHandle, stream_id: i32, msg: MSG) {
        let imp = &mut *self.imp.borrow_mut();

        let send_msg = NetData::<MSG> {
            stream_id,
            msg
        };

        let encoded = serde_wasm_bindgen::to_value(&send_msg).unwrap();
        
        match imp.handle_map.get(&handle) {
            Some(info) => {
                match &info._dc {
                    Some(dc) => {
                        log(&format!("Net(send) handle:{}, msg: {:?}", handle, &send_msg));
                        dc.send(&encoded);
                    },
                    None => {
                        log(&format!("Net(send) handle:{} No DC", handle));
                    }
                }
            },
            None => {
                log(&format!("Net(send) handle:{} not found", handle));
            }
        }
    }
    
    /// Return a new stream_id for the specified 'peer'
    pub fn new_stream_id(&mut self, NetworkHandle(handle): NetworkHandle) -> Option<i32> {
        let imp = &mut *self.imp.borrow_mut();

        let info = imp.handle_map.get_mut(&handle)?;
        let ret = info.next_stream_id;
        info.next_stream_id += 2;

        Some(ret)
    }
    
    /// Return all the received updates for the specified 'handle
    pub fn get_handle_events(&mut self, NetworkHandle(handle): NetworkHandle, stream_id: i32) -> Vec<NetUpdate<MSG>> {
        let imp = &mut *self.imp.borrow_mut();

        if let Some(info) = imp.handle_map.get_mut(&handle) {
            if let Some(msgs) = info.received_msgs.get_mut(&stream_id) {
                return std::mem::take(msgs);
            }
        }

        return Vec::new();
    }
}