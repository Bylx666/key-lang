
//mod samples/testmod.ks> m;

let i = 0;
let next() {
  i+=1;
  if i>5:false // 可别加分号
  else: true
}
for(next()) let inner = 9;

log(()+2) // 9