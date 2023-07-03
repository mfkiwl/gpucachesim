#ifndef MAXFLOPS_SFU_DEF_H
#define MAXFLOPS_SFU_DEF_H

#include <algorithm>
#include <cuda.h>
#include <iostream>
#include <stdio.h>
#include <stdlib.h>

#include "../../../hw_def/hw_def.h"

#define REPEAT_TIMES 1024

__global__ void max_flops(uint64_t *startClk, uint64_t *stopClk, float *data1,
                          float *res) {
  int gid = blockIdx.x * blockDim.x + threadIdx.x;
  register float s1 = data1[gid];
  register float result = s1;

  // synchronize all threads
  asm volatile("bar.sync 0;");

  // start timing
  uint64_t start = 0;
  asm volatile("mov.u64 %0, %%clock64;" : "=l"(start)::"memory");

  for (int j = 0; j < REPEAT_TIMES; ++j) {
    asm volatile(
        "{\t\n"
        "sqrt.approx.ftz.f32 %0, %0;\n\t"
        "sqrt.approx.ftz.f32 %0, %0;\n\t"
        "sqrt.approx.ftz.f32 %0, %0;\n\t"
        "sqrt.approx.ftz.f32 %0, %0;\n\t"
        "}"
        : "+f"(result));
  }
  // synchronize all threads
  asm volatile("bar.sync 0;");

  // stop timing
  uint64_t stop = 0;
  asm volatile("mov.u64 %0, %%clock64;" : "=l"(stop)::"memory");

  // write time and data back to memory
  startClk[gid] = start;
  stopClk[gid] = stop;
  res[gid] = result;
}

float sfu_max_flops() {
  intilizeDeviceProp(0);

  BLOCKS_NUM = 1;
  TOTAL_THREADS = THREADS_PER_BLOCK * BLOCKS_NUM;

  uint64_t *startClk = (uint64_t *)malloc(TOTAL_THREADS * sizeof(uint64_t));
  uint64_t *stopClk = (uint64_t *)malloc(TOTAL_THREADS * sizeof(uint64_t));
  float *data1 = (float *)malloc(TOTAL_THREADS * sizeof(float));
  float *res = (float *)malloc(TOTAL_THREADS * sizeof(float));

  uint64_t *startClk_g;
  uint64_t *stopClk_g;
  float *data1_g;
  float *res_g;

  for (uint32_t i = 0; i < TOTAL_THREADS; i++) {
    data1[i] = 987654321.789456 + (float)i;
  }

  gpuErrchk(cudaMalloc(&startClk_g, TOTAL_THREADS * sizeof(uint64_t)));
  gpuErrchk(cudaMalloc(&stopClk_g, TOTAL_THREADS * sizeof(uint64_t)));
  gpuErrchk(cudaMalloc(&data1_g, TOTAL_THREADS * sizeof(float)));
  gpuErrchk(cudaMalloc(&res_g, TOTAL_THREADS * sizeof(float)));

  gpuErrchk(cudaMemcpy(data1_g, data1, TOTAL_THREADS * sizeof(float),
                       cudaMemcpyHostToDevice));

  max_flops<<<BLOCKS_NUM, THREADS_PER_BLOCK>>>(startClk_g, stopClk_g, data1_g,
                                               res_g);
  gpuErrchk(cudaPeekAtLastError());

  gpuErrchk(cudaMemcpy(startClk, startClk_g, TOTAL_THREADS * sizeof(uint64_t),
                       cudaMemcpyDeviceToHost));
  gpuErrchk(cudaMemcpy(stopClk, stopClk_g, TOTAL_THREADS * sizeof(uint64_t),
                       cudaMemcpyDeviceToHost));
  gpuErrchk(cudaMemcpy(res, res_g, TOTAL_THREADS * sizeof(float),
                       cudaMemcpyDeviceToHost));

  float flops;
  flops = (float)(REPEAT_TIMES * TOTAL_THREADS * 4) /
          ((float)(stopClk[0] - startClk[0]));
  std::cout << "SFU fast sqrt bw = " << flops << "(flops/clk/SM) \n";
  std::cout << "Total Clk number = " << (stopClk[0] - startClk[0]) << "\n";

  return flops;
}

#endif