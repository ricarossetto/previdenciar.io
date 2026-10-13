import fs from 'node:fs';
import path from 'node:path';
const endpoint=process.argv[2];
if(!/^http:\/\/127\.0\.0\.1:\d+$/.test(endpoint??''))throw Error('Explicit loopback endpoint required');
const expected=fs.readdirSync('apps/desktop/dist/assets').filter(name=>/\.(js|css)$/.test(name));
if(!expected.some(name=>name.endsWith('.js'))||!expected.some(name=>name.endsWith('.css')))throw Error('Frontend build is incomplete');
let last='Native page not ready',passed=false;
for(let attempt=0;attempt<30;attempt++){
  let socket;
  try{
    const targets=await fetch(endpoint+'/json/list',{signal:AbortSignal.timeout(2000)}).then(r=>r.json());
    const target=targets.find(t=>t.url==='http://tauri.localhost/');if(!target)throw Error('Owned app page not found');
    socket=new WebSocket(target.webSocketDebuggerUrl);await new Promise((resolve,reject)=>{socket.addEventListener('open',resolve,{once:true});socket.addEventListener('error',reject,{once:true});});
    let id=0;const pending=new Map();socket.addEventListener('message',event=>{const value=JSON.parse(event.data);if(pending.has(value.id)){pending.get(value.id)(value);pending.delete(value.id)}});
    const send=method=>new Promise((resolve,reject)=>{const number=++id;const timer=setTimeout(()=>{pending.delete(number);reject(Error('CDP observation timeout'))},3000);pending.set(number,value=>{clearTimeout(timer);value.error?reject(Error(value.error.message)):resolve(value.result)});socket.send(JSON.stringify({id:number,method}));});
    const resources=await send('Page.getResourceTree'),ax=await send('Accessibility.getFullAXTree');
    const urls=resources.frameTree.resources.map(r=>r.url),headings=ax.nodes.filter(n=>!n.ignored&&n.role?.value==='heading').map(n=>n.name?.value);
    if(expected.some(name=>!urls.includes('http://tauri.localhost/assets/'+name))||!headings.includes('Início'))throw Error('Final JS/CSS or real home page not loaded');
    const result={status:'passed',readOnly:true,isolatedProfile:true,expectedAssets:expected,headings,fullNativeWorkflowsCertified:false};
    const output=process.argv[3];if(output){fs.mkdirSync(path.dirname(output),{recursive:true});fs.writeFileSync(output,JSON.stringify(result,null,2));}
    console.log(JSON.stringify(result));await new Promise(resolve=>{socket.addEventListener('close',resolve,{once:true});socket.close()});passed=true;break;
  }catch(error){last=error.message;if(socket)socket.close();await new Promise(resolve=>setTimeout(resolve,1000));}
}
if(!passed)throw Error('Native release rejected: '+last);
