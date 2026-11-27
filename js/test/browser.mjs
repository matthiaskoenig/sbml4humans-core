import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import { readFile } from 'node:fs/promises';
import { resolve,extname } from 'node:path';
import { fileURLToPath } from 'node:url';
const { chromium } = await import(process.env.PLAYWRIGHT_MODULE || 'playwright');
const root=fileURLToPath(new URL('../../', import.meta.url));
const server=createServer(async(request,response)=>{
 try {
  if(request.url==='/'){response.setHeader('content-type','text/html');response.end('<!doctype html><title>SBML worker test</title>');return;}
  const pathname=new URL(request.url,'http://localhost').pathname;
  const path=resolve(root,'.'+pathname+(pathname.endsWith('/')?'index.html':''));
  if(!path.startsWith(root)){response.writeHead(404);response.end();return;}
  response.setHeader('content-type',extname(path)==='.html'?'text/html':extname(path)==='.wasm'?'application/wasm':extname(path)==='.mjs'?'text/javascript':'application/octet-stream');
  response.end(await readFile(path));
 }catch{response.writeHead(404);response.end();}
});
await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));
const browser=await chromium.launch({headless:true});
try {
 const page=await browser.newPage();
 const errors=[];page.on('pageerror',error=>errors.push(error.message));
 await page.goto('http://127.0.0.1:'+server.address().port);
 const result=await page.evaluate(async()=>{
  const {createReader}=await import('/js/index.mjs');
  const reader=await createReader();
  try{
   const source=await (await fetch('/testdata/comp_internal.xml')).arrayBuffer();
   const expected=await (await fetch('/testdata/oracles/comp_internal.json')).json();
   const actual=await reader.report(source);
   if(JSON.stringify(actual)!==JSON.stringify(expected)) {
    // Object key order is irrelevant to the wire contract.
    const sort=value=>Array.isArray(value)?value.map(sort):value&&typeof value==='object'?Object.fromEntries(Object.keys(value).sort().map(key=>[key,sort(value[key])])):value;
    if(JSON.stringify(sort(actual))!==JSON.stringify(sort(expected))) throw new Error('Browser full report differs from Python');
   }
   let ticks=0;const timer=setInterval(()=>ticks++,1);
   const text='<sbml xmlns="http://www.sbml.org/sbml/level3/version2/core" level="3" version="2"><model id="browser"><listOfParameters>'+Array.from({length:3000},(_,i)=>`<parameter id="p${i}" value="${i}" constant="true"/>`).join('')+'</listOfParameters></model></sbml>';
   const report=await reader.report(text);clearInterval(timer);
   const control=new AbortController();const cancelled=reader.report(text,{signal:control.signal});control.abort();
   let cancellation;try{await cancelled;}catch(error){cancellation=error.name;}
   const next=await reader.report('<sbml xmlns="http://www.sbml.org/sbml/level3/version2/core" level="3" version="2"><model id="after"/></sbml>');
   return{ticks,count:report.models[0].listOfParameters.length,nodes:Object.keys(report.linkGraph.nodes).length,cancellation,next:next.models[0].id};
  }finally{reader.dispose();}
 });
 assert.equal(result.count,3000);assert.equal(result.nodes,3002);assert.ok(result.ticks>0);assert.equal(result.cancellation,'AbortError');assert.equal(result.next,'after');assert.deepEqual(errors,[]);
 await page.goto('http://127.0.0.1:'+server.address().port+'/js/example/');
 await page.getByTestId('status').filter({hasText:'Ready'}).waitFor();
 await page.getByRole('button',{name:'Load example'}).click();
 await page.getByTestId('status').filter({hasText:'Ready:'}).waitFor();
 assert.equal(await page.getByRole('button',{name:'Download report'}).isEnabled(),true);
 if(process.env.SBML_CORE_SCREENSHOT) await page.screenshot({path:process.env.SBML_CORE_SCREENSHOT,fullPage:true});
 console.log('Chromium WASM worker:',JSON.stringify(result));
}finally{await browser.close();await new Promise(resolve=>server.close(resolve));}
