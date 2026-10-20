/* incremental min-degree scan from adjacency, k=7, n up to N; prints admissible intervals */
#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
int main(int argc,char**argv){int k=atoi(argv[1]);uint64_t N=strtoull(argv[2],0,10);
 uint64_t pw[64];int np=0;for(uint64_t b=1;;b++){unsigned __int128 p=1;for(int i=0;i<k;i++)p*=b;if(p>2*N+2)break;pw[np++]=(uint64_t)p;}
 uint8_t*deg=calloc(N+2,1);int64_t low=0;uint64_t st=0;int in=0;
 for(uint64_t n=1;n<=N;n++){int d=0;for(int i=0;i<np;i++){uint64_t p=pw[i];if(p<=n)continue;if(p>=2*n)break;uint64_t u=p-n;d++;
   if(deg[u]<2){deg[u]++;if(deg[u]==2)low--;}}
  if(d>2)d=2;deg[n]=d;if(d<2)low++;
  if(low==0&&!in){in=1;st=n;} if(low!=0&&in){in=0;printf("[%llu,%llu]\n",(unsigned long long)st,(unsigned long long)n-1);} }
 if(in)printf("[%llu,%llu]\n",(unsigned long long)st,(unsigned long long)N);return 0;}
