// Only nonsensitive edit choices live here. Runtime confirmations, passwords,
// source content and approved plans stay in the active interaction.
export function readTaskDraft<T extends Record<string,unknown>>(key:string,defaults:T):T {
  try {
    const value=JSON.parse(localStorage.getItem(key) ?? '{}');
    if(!value || Array.isArray(value) || typeof value!=='object')return defaults;
    const result={...defaults};
    for(const name of Object.keys(defaults) as (keyof T)[]) {
      const original=defaults[name],item=value[name];
      if(Array.isArray(original) && Array.isArray(item))result[name]=item.filter(x=>typeof x==='string' && original.includes(x)) as T[keyof T];
      else if(typeof original===typeof item && (typeof item!=='string' || item.length<=4096))result[name]=item;
    }
    return result;
  }catch{return defaults}
}
export function saveTaskDraft(key:string,value:Record<string,unknown>) {
  try{localStorage.setItem(key,JSON.stringify(value))}catch{/* Storage failure never grants or blocks execution. */}
}
