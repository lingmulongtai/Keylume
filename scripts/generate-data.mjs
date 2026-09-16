import { mkdirSync, writeFileSync } from 'node:fs';
mkdirSync('resources', { recursive: true });
const leds = [];
for (const [row, y, start, drum] of [['top', 113, 0x60, [40,41,42,43,48,49,50,51]], ['bottom', 165, 0x70, [36,37,38,39,44,45,46,47]]]) {
  for (let i=0;i<8;i++) leds.push({id:`pad.${row}.${i+1}`,kind:'rgb',group:'pads',pos:{x:230+i*51,y},size:{w:41,h:40},address:{dawNote:start+i,drumNote:drum[i],sysexId:start+i},verified:false});
}
for(let i=0;i<9;i++) leds.push({id:`fbtn.${i+1}`,kind:'rgb',group:'faderButtons',pos:{x:670+i*37,y:183},size:{w:24,h:16},address:{cc:0x25+i,sysexId:0x25+i},verified:false});
const buttons = [['shift',63,36,171],['trackPrevious',103,100,171],['trackNext',102,133,171],['encoderUp',51,176,61],['encoderDown',52,176,87],['padUp',106,176,123],['padDown',107,176,153],['scene',104,176,186],['function',105,628,183],['capture',74,690,44],['quantise',75,728,44],['metronome',76,766,44],['undo',77,804,44],['play',115,842,44],['stop',116,880,44],['record',117,918,44],['loop',118,956,44]];
for(const [id,cc,x,y] of buttons) leds.push({id:`btn.${id}`,kind:'mono',group:'buttons',pos:{x,y},size:{w:24,h:15},address:{cc,sysexId:cc,monoStatus:179},verified:false});
let white = 0; const keys=[];
for(let note=36;note<=96;note++){ const black=[1,3,6,8,10].includes(note%12); keys.push({note,x:black?124+white*24-7:124+white*24,w:black?14:23,black}); if(!black)white++; }
const layout={schema:1,model:'launchkey-mk4-61',canvas:{w:1040,h:400},leds,decor:{keys,encoders:Array.from({length:8},(_,i)=>({x:250+i*51,y:72,r:13})),faders:Array.from({length:9},(_,i)=>({x:682+i*37,y:85,h:82})),display:{x:72,y:60,w:91,h:52},wheels:[{x:39,y:254,w:23,h:89},{x:73,y:254,w:23,h:89}]}};
const layer=(effect,params={},blend='normal',zone='all')=>({id:crypto.randomUUID(),effect,params,zone,opacity:1,blend,enabled:true});
const definitions=[
 ['aurora','オーロラ',[layer('aurora',{colors:['#43ffc2','#38a7bf','#a574f9'],speed:0.6})]],
 ['spectrum-wave','スペクトラムウェーブ',[layer('wave',{speed:0.5,wavelength:0.8})]],
 ['white-breeze','ホワイトブリーズ',[layer('breathing',{color:'#fff0d8',period:6})]],
 ['sunset','サンセット',[layer('gradient',{colors:['#ff883e','#f65c9d','#8a5bf7'],angle:0})]],
 ['neon','ネオンリアクティブ',[layer('ripple',{color:'#42fbe5',decay:2,speed:1},'add'),layer('static',{color:'#091635'})]],
 ['starlight','星空',[layer('starlight',{color:'#c1d7ff',density:0.3,speed:0.7},'add'),layer('static',{color:'#07172b'})]],
 ['fire','ファイア',[layer('fire',{speed:1,intensity:1})]],
 ['audio-bars','オーディオバー',[layer('audio_spectrum',{color:'#52f5be',sensitivity:1})]],
 ['beat','ビート',[layer('audio_pulse',{color:'#ff7cb7',sensitivity:1.8},'add'),layer('static',{color:'#10091b'})]],
 ['note-rainbow','キーボードレインボー',[layer('note_map',{},'add'),layer('static',{color:'#090d17'})]],
 ['night','夜間',[layer('static',{color:'#ffd3a3'})]],
 ['power-save','省電力',[layer('hardware_fx',{palette:76,mode:'pulse'})]],
 ['vegas','本体デモ',[]]
];
const presets=definitions.map(([id,name,layers])=>({schema:1,id,name,layers,post:{brightness:id==='night'?0.15:0.8,saturation:1,temperatureK:6500,gamma:2.2},display:{enabled:false,widget:'presetName',showOnPresetChange:true},builtin:true}));
// Approximate preview palette; these colors are explicitly not hardware measurements.
const palette=Array.from({length:128},(_,i)=>i===0?[0,0,0]:i===1?[32,32,32]:i===2?[127,127,127]:i===3?[255,255,255]:hsv((i-4)/124));
function hsv(h){return [0,2,1].map(k=>Math.round(255*(1-Math.max(0,Math.min(1,Math.min((k+h*6)%6,4-(k+h*6)%6))))));}
for(const [file,data] of Object.entries({layout,presets,palette})) writeFileSync(`resources/${file}.json`,JSON.stringify(data,null,2)+'\n');
