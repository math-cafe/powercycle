/* chk N FILE: is FILE a Hamiltonian cycle of G_2(N)? exact integer checks */
#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
static int issq(uint64_t s){ uint64_t r=(uint64_t)__builtin_sqrtl((long double)s); while(r*r>s) r--; while((r+1)*(r+1)<=s) r++; return r*r==s; }
int main(int c,char**v){ uint64_t n=strtoull(v[1],0,10); FILE*f=fopen(v[2],"r"); if(!f) return 3;
 char*seen=calloc(n+1,1); uint64_t x,first=0,prev=0,cnt=0;
 while(fscanf(f,"%llu",(unsigned long long*)&x)==1){ if(x<1||x>n||seen[x]){printf("BAD value %llu\n",(unsigned long long)x);return 1;} seen[x]=1;
  if(cnt==0) first=x; else if(!issq(prev+x)){printf("BAD sum %llu+%llu at %llu\n",(unsigned long long)prev,(unsigned long long)x,(unsigned long long)cnt);return 1;}
  prev=x; cnt++; }
 if(cnt!=n){printf("BAD count %llu\n",(unsigned long long)cnt);return 1;}
 if(!issq(prev+first)){printf("BAD closing %llu+%llu\n",(unsigned long long)prev,(unsigned long long)first);return 1;}
 printf("OK\n"); return 0; }
