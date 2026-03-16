git (terrain):
t0: 8.49 MiB (16.87 MiB)
t1: 12.04 MiB (32.11 MiB)

# ONE TNT

git (no terrain, git repack -a -d --depth 4095 --window 64)
t0: 7.19 MiB in 4.281s
t1: 12.83 MiB in 4.271s

git (no terrain, git gc --aggressive)
t0: 7.19 MiB in 27.285s
t1: 10.25 MiB in 39.947s

git (no terrain, git repack -d)
t0: 7.17 MiB in 1.564s
t1: 13.01 MiB in 1.274s

# test42

## git (no terrain, git repack -d)

0 /tmp/repo-base-dir/0
Enumerating objects: 8176, done.
Counting objects: 100% (8176/8176), done.
Delta compression using up to 16 threads
Compressing objects: 100% (8166/8166), done.
Writing objects: 100% (8176/8176), done.
Total 8176 (delta 5762), reused 0 (delta 0), pack-reused 0 (from 0)
#0

count: 0
size: 0 bytes
in-pack: 16352
packs: 2
size-pack: 53.66 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 6.9926s

---

1 /tmp/repo-base-dir/1
Enumerating objects: 2726, done.
Counting objects: 100% (2726/2726), done.
Delta compression using up to 16 threads
Compressing objects: 100% (2717/2717), done.
Writing objects: 100% (2726/2726), done.
Total 2726 (delta 1591), reused 0 (delta 0), pack-reused 0 (from 0)
#1

count: 0
size: 0 bytes
in-pack: 19078
packs: 3
size-pack: 60.19 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 1.2555s

---

2 /tmp/repo-base-dir/2
Enumerating objects: 2559, done.
Counting objects: 100% (2559/2559), done.
Delta compression using up to 16 threads
Compressing objects: 100% (2555/2555), done.
Writing objects: 100% (2559/2559), done.
Total 2559 (delta 1498), reused 0 (delta 0), pack-reused 0 (from 0)
#2

count: 0
size: 0 bytes
in-pack: 21637
packs: 4
size-pack: 63.33 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 0.5648s

---

3 /tmp/repo-base-dir/3
Enumerating objects: 1952, done.
Counting objects: 100% (1952/1952), done.
Delta compression using up to 16 threads
Compressing objects: 100% (1948/1948), done.
Writing objects: 100% (1952/1952), done.
Total 1952 (delta 1107), reused 0 (delta 0), pack-reused 0 (from 0)
#3

count: 0
size: 0 bytes
in-pack: 23589
packs: 5
size-pack: 65.21 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 0.3300s

---

4 /tmp/repo-base-dir/4
Enumerating objects: 1418, done.
Counting objects: 100% (1418/1418), done.
Delta compression using up to 16 threads
Compressing objects: 100% (1414/1414), done.
Writing objects: 100% (1418/1418), done.
Total 1418 (delta 865), reused 0 (delta 0), pack-reused 0 (from 0)
#4

count: 0
size: 0 bytes
in-pack: 25007
packs: 6
size-pack: 66.58 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 0.2792s

---

5 /tmp/repo-base-dir/5
Enumerating objects: 1229, done.
Counting objects: 100% (1229/1229), done.
Delta compression using up to 16 threads
Compressing objects: 100% (1225/1225), done.
Writing objects: 100% (1229/1229), done.
Total 1229 (delta 733), reused 0 (delta 0), pack-reused 0 (from 0)
#5

count: 0
size: 0 bytes
in-pack: 26236
packs: 7
size-pack: 67.73 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 0.2252s

---

6 /tmp/repo-base-dir/6
Enumerating objects: 1446, done.
Counting objects: 100% (1446/1446), done.
Delta compression using up to 16 threads
Compressing objects: 100% (1441/1441), done.
Writing objects: 100% (1446/1446), done.
Total 1446 (delta 690), reused 0 (delta 0), pack-reused 0 (from 0)
#6

count: 0
size: 0 bytes
in-pack: 27682
packs: 8
size-pack: 68.98 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 0.2207s

---

7 /tmp/repo-base-dir/7
Enumerating objects: 1, done.
Counting objects: 100% (1/1), done.
Writing objects: 100% (1/1), done.
Total 1 (delta 0), reused 0 (delta 0), pack-reused 0 (from 0)
#7

count: 0
size: 0 bytes
in-pack: 27683
packs: 9
size-pack: 68.98 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 0.0084s

---

8 /tmp/repo-base-dir/8
Enumerating objects: 1825, done.
Counting objects: 100% (1825/1825), done.
Delta compression using up to 16 threads
Compressing objects: 100% (1821/1821), done.
Writing objects: 100% (1825/1825), done.
Total 1825 (delta 1012), reused 0 (delta 0), pack-reused 0 (from 0)
#8

count: 0
size: 0 bytes
in-pack: 29508
packs: 10
size-pack: 70.60 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 0.2804s

---

9 /tmp/repo-base-dir/9
Enumerating objects: 2374, done.
Counting objects: 100% (2374/2374), done.
Delta compression using up to 16 threads
Compressing objects: 100% (2367/2367), done.
Writing objects: 100% (2374/2374), done.
Total 2374 (delta 1414), reused 0 (delta 0), pack-reused 0 (from 0)
#9

count: 0
size: 0 bytes
in-pack: 31882
packs: 11
size-pack: 73.48 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 0.4797s

---

10 /tmp/repo-base-dir/10
Enumerating objects: 2622, done.
Counting objects: 100% (2622/2622), done.
Delta compression using up to 16 threads
Compressing objects: 100% (2618/2618), done.
Writing objects: 100% (2622/2622), done.
Total 2622 (delta 1519), reused 0 (delta 0), pack-reused 0 (from 0)
#10

count: 0
size: 0 bytes
in-pack: 34504
packs: 12
size-pack: 76.43 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 0.4844s

---

11 /tmp/repo-base-dir/11
Enumerating objects: 2609, done.
Counting objects: 100% (2609/2609), done.
Delta compression using up to 16 threads
Compressing objects: 100% (2605/2605), done.
Writing objects: 100% (2609/2609), done.
Total 2609 (delta 1558), reused 0 (delta 0), pack-reused 0 (from 0)
#11

count: 0
size: 0 bytes
in-pack: 37113
packs: 13
size-pack: 79.70 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 0.5424s

---

12 /tmp/repo-base-dir/12
Enumerating objects: 2573, done.
Counting objects: 100% (2573/2573), done.
Delta compression using up to 16 threads
Compressing objects: 100% (2569/2569), done.
Writing objects: 100% (2573/2573), done.
Total 2573 (delta 1562), reused 0 (delta 0), pack-reused 0 (from 0)
#12

count: 0
size: 0 bytes
in-pack: 39686
packs: 14
size-pack: 82.72 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 0.5012s

---

13 /tmp/repo-base-dir/13
Enumerating objects: 1249, done.
Counting objects: 100% (1249/1249), done.
Delta compression using up to 16 threads
Compressing objects: 100% (1245/1245), done.
Writing objects: 100% (1249/1249), done.
Total 1249 (delta 758), reused 0 (delta 0), pack-reused 0 (from 0)
#13

count: 0
size: 0 bytes
in-pack: 40935
packs: 15
size-pack: 83.74 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 0.1987s

---

## git (no terrain, git gc --aggressive)

0 /tmp/repo-base-dir/0
Enumerating objects: 8176, done.
Counting objects: 100% (8176/8176), done.
Delta compression using up to 16 threads
Compressing objects: 100% (8166/8166), done.
Writing objects: 100% (8176/8176), done.
Building bitmaps: 100% (1/1), done.
Total 8176 (delta 6268), reused 0 (delta 0), pack-reused 0 (from 0)
#0

count: 0
size: 0 bytes
in-pack: 8176
packs: 1
size-pack: 26.83 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 42.3203s

---

1 /tmp/repo-base-dir/1
Enumerating objects: 10902, done.
Counting objects: 100% (10902/10902), done.
Delta compression using up to 16 threads
Compressing objects: 100% (10883/10883), done.
Writing objects: 100% (10902/10902), done.
Building bitmaps: 100% (2/2), done.
Total 10902 (delta 9026), reused 917 (delta 0), pack-reused 0 (from 0)
#1

count: 0
size: 0 bytes
in-pack: 10902
packs: 1
size-pack: 27.49 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 39.4194s

---

2 /tmp/repo-base-dir/2
Enumerating objects: 13461, done.
Counting objects: 100% (13461/13461), done.
Delta compression using up to 16 threads
Compressing objects: 100% (13438/13438), done.
Writing objects: 100% (13461/13461), done.
Building bitmaps: 100% (3/3), done.
Total 13461 (delta 11537), reused 1022 (delta 0), pack-reused 0 (from 0)
#2

count: 0
size: 0 bytes
in-pack: 13461
packs: 1
size-pack: 27.84 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 38.6721s

---

3 /tmp/repo-base-dir/3
Enumerating objects: 15413, done.
Counting objects: 100% (15413/15413), done.
Delta compression using up to 16 threads
Compressing objects: 100% (15386/15386), done.
Writing objects: 100% (15413/15413), done.
Building bitmaps: 100% (4/4), done.
Total 15413 (delta 13451), reused 849 (delta 0), pack-reused 0 (from 0)
#3

count: 0
size: 0 bytes
in-pack: 15413
packs: 1
size-pack: 28.05 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 42.6380s

---

4 /tmp/repo-base-dir/4
Enumerating objects: 16831, done.
Counting objects: 100% (16831/16831), done.
Delta compression using up to 16 threads
Compressing objects: 100% (16800/16800), done.
Writing objects: 100% (16831/16831), done.
Building bitmaps: 100% (5/5), done.
Total 16831 (delta 14832), reused 1543 (delta 0), pack-reused 0 (from 0)
#4

count: 0
size: 0 bytes
in-pack: 16831
packs: 1
size-pack: 28.23 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 41.8042s

---

5 /tmp/repo-base-dir/5
Enumerating objects: 18060, done.
Counting objects: 100% (18060/18060), done.
Delta compression using up to 16 threads
Compressing objects: 100% (18025/18025), done.
Writing objects: 100% (18060/18060), done.
Building bitmaps: 100% (6/6), done.
Total 18060 (delta 16021), reused 1608 (delta 0), pack-reused 0 (from 0)
#5

count: 0
size: 0 bytes
in-pack: 18060
packs: 1
size-pack: 28.34 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 41.9131s

---

6 /tmp/repo-base-dir/6
Enumerating objects: 19506, done.
Counting objects: 100% (19506/19506), done.
Delta compression using up to 16 threads
Compressing objects: 100% (19466/19466), done.
Writing objects: 100% (19506/19506), done.
Building bitmaps: 100% (7/7), done.
Total 19506 (delta 17444), reused 1365 (delta 0), pack-reused 0 (from 0)
#6

count: 0
size: 0 bytes
in-pack: 19506
packs: 1
size-pack: 28.55 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 44.0312s

---

7 /tmp/repo-base-dir/7
Enumerating objects: 19507, done.
Counting objects: 100% (19507/19507), done.
Delta compression using up to 16 threads
Compressing objects: 100% (19467/19467), done.
Writing objects: 100% (19507/19507), done.
Building bitmaps: 100% (8/8), done.
Total 19507 (delta 17438), reused 1460 (delta 0), pack-reused 0 (from 0)
#7

count: 0
size: 0 bytes
in-pack: 19507
packs: 1
size-pack: 28.56 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 44.0197s

---

8 /tmp/repo-base-dir/8
Enumerating objects: 21332, done.
Counting objects: 100% (21332/21332), done.
Delta compression using up to 16 threads
Compressing objects: 100% (21288/21288), done.
Writing objects: 100% (21332/21332), done.
Building bitmaps: 100% (9/9), done.
Total 21332 (delta 19211), reused 1467 (delta 0), pack-reused 0 (from 0)
#8

count: 0
size: 0 bytes
in-pack: 21332
packs: 1
size-pack: 28.79 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 29.9279s

---

9 /tmp/repo-base-dir/9
Enumerating objects: 23706, done.
Counting objects: 100% (23706/23706), done.
Delta compression using up to 16 threads
Compressing objects: 100% (23655/23655), done.
Writing objects: 100% (23706/23706), done.
Building bitmaps: 100% (10/10), done.
Total 23706 (delta 21525), reused 1453 (delta 0), pack-reused 0 (from 0)
#9

count: 0
size: 0 bytes
in-pack: 23706
packs: 1
size-pack: 29.32 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 33.7334s

---

10 /tmp/repo-base-dir/10
Enumerating objects: 26328, done.
Counting objects: 100% (26328/26328), done.
Delta compression using up to 16 threads
Compressing objects: 100% (26273/26273), done.
Writing objects: 100% (26328/26328), done.
Building bitmaps: 100% (11/11), done.
Total 26328 (delta 24098), reused 1361 (delta 0), pack-reused 0 (from 0)
#10

count: 0
size: 0 bytes
in-pack: 26328
packs: 1
size-pack: 29.70 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 34.3395s

---

11 /tmp/repo-base-dir/11
Enumerating objects: 28937, done.
Counting objects: 100% (28937/28937), done.
Delta compression using up to 16 threads
Compressing objects: 100% (28878/28878), done.
Writing objects: 100% (28937/28937), done.
Building bitmaps: 100% (12/12), done.
Total 28937 (delta 26660), reused 1464 (delta 0), pack-reused 0 (from 0)
#11

count: 0
size: 0 bytes
in-pack: 28937
packs: 1
size-pack: 30.15 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 36.2308s

---

12 /tmp/repo-base-dir/12
Enumerating objects: 31510, done.
Counting objects: 100% (31510/31510), done.
Delta compression using up to 16 threads
Compressing objects: 100% (31447/31447), done.
Writing objects: 100% (31510/31510), done.
Building bitmaps: 100% (13/13), done.
Total 31510 (delta 29184), reused 1550 (delta 0), pack-reused 0 (from 0)
#12

count: 0
size: 0 bytes
in-pack: 31510
packs: 1
size-pack: 30.58 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 36.8728s

---

13 /tmp/repo-base-dir/13
Enumerating objects: 32759, done.
Counting objects: 100% (32759/32759), done.
Delta compression using up to 16 threads
Compressing objects: 100% (32692/32692), done.
Writing objects: 100% (32759/32759), done.
Building bitmaps: 100% (14/14), done.
Total 32759 (delta 30407), reused 1975 (delta 0), pack-reused 0 (from 0)
#13

count: 0
size: 0 bytes
in-pack: 32759
packs: 1
size-pack: 30.70 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 37.8910s

---

## git (no terrain, git repack -a -d --depth 4095 --window 256)

0 /tmp/repo-base-dir/0
Enumerating objects: 8176, done.
Counting objects: 100% (8176/8176), done.
Delta compression using up to 16 threads
Compressing objects: 100% (8166/8166), done.
Writing objects: 100% (8176/8176), done.
Building bitmaps: 100% (1/1), done.
Total 8176 (delta 6291), reused 0 (delta 0), pack-reused 0 (from 0)
#0

count: 0
size: 0 bytes
in-pack: 8176
packs: 1
size-pack: 26.83 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 46.6980s

---

1 /tmp/repo-base-dir/1
Enumerating objects: 10902, done.
Counting objects: 100% (10902/10902), done.
Delta compression using up to 16 threads
Compressing objects: 100% (4592/4592), done.
Writing objects: 100% (10902/10902), done.
Building bitmaps: 100% (2/2), done.
Total 10902 (delta 9097), reused 7203 (delta 6291), pack-reused 0 (from 0)
#1

count: 0
size: 0 bytes
in-pack: 10902
packs: 1
size-pack: 32.66 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 20.4435s

---

2 /tmp/repo-base-dir/2
Enumerating objects: 13461, done.
Counting objects: 100% (13461/13461), done.
Delta compression using up to 16 threads
Compressing objects: 100% (4341/4341), done.
Writing objects: 100% (13461/13461), done.
Building bitmaps: 100% (3/3), done.
Total 13461 (delta 11611), reused 10103 (delta 9097), pack-reused 0 (from 0)
#2

count: 0
size: 0 bytes
in-pack: 13461
packs: 1
size-pack: 35.16 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 18.4235s

---

3 /tmp/repo-base-dir/3
Enumerating objects: 15413, done.
Counting objects: 100% (15413/15413), done.
Delta compression using up to 16 threads
Compressing objects: 100% (3775/3775), done.
Writing objects: 100% (15413/15413), done.
Building bitmaps: 100% (4/4), done.
Total 15413 (delta 13529), reused 12816 (delta 11611), pack-reused 0 (from 0)
#3

count: 0
size: 0 bytes
in-pack: 15413
packs: 1
size-pack: 36.47 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 4.3009s

---

4 /tmp/repo-base-dir/4
Enumerating objects: 16831, done.
Counting objects: 100% (16831/16831), done.
Delta compression using up to 16 threads
Compressing objects: 100% (3271/3271), done.
Writing objects: 100% (16831/16831), done.
Building bitmaps: 100% (5/5), done.
Total 16831 (delta 14937), reused 15026 (delta 13529), pack-reused 0 (from 0)
#4

count: 0
size: 0 bytes
in-pack: 16831
packs: 1
size-pack: 37.45 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 2.5598s

---

5 /tmp/repo-base-dir/5
Enumerating objects: 18060, done.
Counting objects: 100% (18060/18060), done.
Delta compression using up to 16 threads
Compressing objects: 100% (3088/3088), done.
Writing objects: 100% (18060/18060), done.
Building bitmaps: 100% (6/6), done.
Total 18060 (delta 16143), reused 16498 (delta 14937), pack-reused 0 (from 0)
#5

count: 0
size: 0 bytes
in-pack: 18060
packs: 1
size-pack: 38.23 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 1.5957s

---

6 /tmp/repo-base-dir/6
Enumerating objects: 19506, done.
Counting objects: 100% (19506/19506), done.
Delta compression using up to 16 threads
Compressing objects: 100% (3323/3323), done.
Writing objects: 100% (19506/19506), done.
Building bitmaps: 100% (7/7), done.
Total 19506 (delta 17564), reused 17528 (delta 16143), pack-reused 0 (from 0)
#6

count: 0
size: 0 bytes
in-pack: 19506
packs: 1
size-pack: 38.95 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 0.7400s

---

7 /tmp/repo-base-dir/7
Enumerating objects: 19507, done.
Counting objects: 100% (19507/19507), done.
Delta compression using up to 16 threads
Compressing objects: 100% (1903/1903), done.
Writing objects: 100% (19507/19507), done.
Building bitmaps: 100% (8/8), done.
Total 19507 (delta 17564), reused 19506 (delta 17564), pack-reused 0 (from 0)
#7

count: 0
size: 0 bytes
in-pack: 19507
packs: 1
size-pack: 38.95 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 0.1138s

---

8 /tmp/repo-base-dir/8
Enumerating objects: 21332, done.
Counting objects: 100% (21332/21332), done.
Delta compression using up to 16 threads
Compressing objects: 100% (3724/3724), done.
Writing objects: 100% (21332/21332), done.
Building bitmaps: 100% (9/9), done.
Total 21332 (delta 19346), reused 18949 (delta 17564), pack-reused 0 (from 0)
#8

count: 0
size: 0 bytes
in-pack: 21332
packs: 1
size-pack: 40.02 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 2.4399s

---

9 /tmp/repo-base-dir/9
Enumerating objects: 23706, done.
Counting objects: 100% (23706/23706), done.
Delta compression using up to 16 threads
Compressing objects: 100% (4309/4309), done.
Writing objects: 100% (23706/23706), done.
Building bitmaps: 100% (10/10), done.
Total 23706 (delta 21664), reused 20706 (delta 19346), pack-reused 0 (from 0)
#9

count: 0
size: 0 bytes
in-pack: 23706
packs: 1
size-pack: 42.30 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 15.4112s

---

10 /tmp/repo-base-dir/10
Enumerating objects: 26328, done.
Counting objects: 100% (26328/26328), done.
Delta compression using up to 16 threads
Compressing objects: 100% (4609/4609), done.
Writing objects: 100% (26328/26328), done.
Building bitmaps: 100% (11/11), done.
Total 26328 (delta 24262), reused 22920 (delta 21664), pack-reused 0 (from 0)
#10

count: 0
size: 0 bytes
in-pack: 26328
packs: 1
size-pack: 44.53 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 13.7863s

---

11 /tmp/repo-base-dir/11
Enumerating objects: 28937, done.
Counting objects: 100% (28937/28937), done.
Delta compression using up to 16 threads
Compressing objects: 100% (4616/4616), done.
Writing objects: 100% (28937/28937), done.
Building bitmaps: 100% (12/12), done.
Total 28937 (delta 26829), reused 25595 (delta 24262), pack-reused 0 (from 0)
#11

count: 0
size: 0 bytes
in-pack: 28937
packs: 1
size-pack: 47.13 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 18.2811s

---

12 /tmp/repo-base-dir/12
Enumerating objects: 31510, done.
Counting objects: 100% (31510/31510), done.
Delta compression using up to 16 threads
Compressing objects: 100% (4618/4618), done.
Writing objects: 100% (31510/31510), done.
Building bitmaps: 100% (13/13), done.
Total 31510 (delta 29358), reused 28235 (delta 26829), pack-reused 0 (from 0)
#12

count: 0
size: 0 bytes
in-pack: 31510
packs: 1
size-pack: 49.49 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 15.4392s

---

13 /tmp/repo-base-dir/13
Enumerating objects: 32759, done.
Counting objects: 100% (32759/32759), done.
Delta compression using up to 16 threads
Compressing objects: 100% (3334/3334), done.
Writing objects: 100% (32759/32759), done.
Building bitmaps: 100% (14/14), done.
Total 32759 (delta 30594), reused 31192 (delta 29358), pack-reused 0 (from 0)
#13

count: 0
size: 0 bytes
in-pack: 32759
packs: 1
size-pack: 50.13 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 0.9871s

---

## git (no terrain, git repack -a -d --depth 4095 --window 128)

0 /tmp/repo-base-dir/0
Enumerating objects: 8176, done.
Counting objects: 100% (8176/8176), done.
Delta compression using up to 16 threads
Compressing objects: 100% (8166/8166), done.
Writing objects: 100% (8176/8176), done.
Building bitmaps: 100% (1/1), done.
Total 8176 (delta 6250), reused 0 (delta 0), pack-reused 0 (from 0)
#0

count: 0
size: 0 bytes
in-pack: 8176
packs: 1
size-pack: 26.80 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 19.4635s

---

1 /tmp/repo-base-dir/1
Enumerating objects: 10902, done.
Counting objects: 100% (10902/10902), done.
Delta compression using up to 16 threads
Compressing objects: 100% (4633/4633), done.
Writing objects: 100% (10902/10902), done.
Building bitmaps: 100% (2/2), done.
Total 10902 (delta 9025), reused 7186 (delta 6250), pack-reused 0 (from 0)
#1

count: 0
size: 0 bytes
in-pack: 10902
packs: 1
size-pack: 32.64 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 8.9839s

---

2 /tmp/repo-base-dir/2
Enumerating objects: 13461, done.
Counting objects: 100% (13461/13461), done.
Delta compression using up to 16 threads
Compressing objects: 100% (4413/4413), done.
Writing objects: 100% (13461/13461), done.
Building bitmaps: 100% (3/3), done.
Total 13461 (delta 11519), reused 10091 (delta 9025), pack-reused 0 (from 0)
#2

count: 0
size: 0 bytes
in-pack: 13461
packs: 1
size-pack: 35.14 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 5.4618s

---

3 /tmp/repo-base-dir/3
Enumerating objects: 15413, done.
Counting objects: 100% (15413/15413), done.
Delta compression using up to 16 threads
Compressing objects: 100% (3867/3867), done.
Writing objects: 100% (15413/15413), done.
Building bitmaps: 100% (4/4), done.
Total 15413 (delta 13455), reused 12789 (delta 11519), pack-reused 0 (from 0)
#3

count: 0
size: 0 bytes
in-pack: 15413
packs: 1
size-pack: 36.43 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 4.2051s

---

4 /tmp/repo-base-dir/4
Enumerating objects: 16831, done.
Counting objects: 100% (16831/16831), done.
Delta compression using up to 16 threads
Compressing objects: 100% (3345/3345), done.
Writing objects: 100% (16831/16831), done.
Building bitmaps: 100% (5/5), done.
Total 16831 (delta 14858), reused 15020 (delta 13455), pack-reused 0 (from 0)
#4

count: 0
size: 0 bytes
in-pack: 16831
packs: 1
size-pack: 37.41 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 2.4519s

---

5 /tmp/repo-base-dir/5
Enumerating objects: 18060, done.
Counting objects: 100% (18060/18060), done.
Delta compression using up to 16 threads
Compressing objects: 100% (3167/3167), done.
Writing objects: 100% (18060/18060), done.
Building bitmaps: 100% (6/6), done.
Total 18060 (delta 16062), reused 16492 (delta 14858), pack-reused 0 (from 0)
#5

count: 0
size: 0 bytes
in-pack: 18060
packs: 1
size-pack: 38.20 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 1.5025s

---

6 /tmp/repo-base-dir/6
Enumerating objects: 19506, done.
Counting objects: 100% (19506/19506), done.
Delta compression using up to 16 threads
Compressing objects: 100% (3404/3404), done.
Writing objects: 100% (19506/19506), done.
Building bitmaps: 100% (7/7), done.
Total 19506 (delta 17474), reused 17520 (delta 16062), pack-reused 0 (from 0)
#6

count: 0
size: 0 bytes
in-pack: 19506
packs: 1
size-pack: 38.90 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 0.6436s

---

7 /tmp/repo-base-dir/7
Enumerating objects: 19507, done.
Counting objects: 100% (19507/19507), done.
Delta compression using up to 16 threads
Compressing objects: 100% (1993/1993), done.
Writing objects: 100% (19507/19507), done.
Building bitmaps: 100% (8/8), done.
Total 19507 (delta 17474), reused 19506 (delta 17474), pack-reused 0 (from 0)
#7

count: 0
size: 0 bytes
in-pack: 19507
packs: 1
size-pack: 38.90 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 0.1161s

---

8 /tmp/repo-base-dir/8
Enumerating objects: 21332, done.
Counting objects: 100% (21332/21332), done.
Delta compression using up to 16 threads
Compressing objects: 100% (3814/3814), done.
Writing objects: 100% (21332/21332), done.
Building bitmaps: 100% (9/9), done.
Total 21332 (delta 19251), reused 18936 (delta 17474), pack-reused 0 (from 0)
#8

count: 0
size: 0 bytes
in-pack: 21332
packs: 1
size-pack: 39.96 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 1.4116s

---

9 /tmp/repo-base-dir/9
Enumerating objects: 23706, done.
Counting objects: 100% (23706/23706), done.
Delta compression using up to 16 threads
Compressing objects: 100% (4404/4404), done.
Writing objects: 100% (23706/23706), done.
Building bitmaps: 100% (10/10), done.
Total 23706 (delta 21569), reused 20684 (delta 19251), pack-reused 0 (from 0)
#9

count: 0
size: 0 bytes
in-pack: 23706
packs: 1
size-pack: 42.23 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 9.2911s

---

10 /tmp/repo-base-dir/10
Enumerating objects: 26328, done.
Counting objects: 100% (26328/26328), done.
Delta compression using up to 16 threads
Compressing objects: 100% (4704/4704), done.
Writing objects: 100% (26328/26328), done.
Building bitmaps: 100% (11/11), done.
Total 26328 (delta 24152), reused 22898 (delta 21569), pack-reused 0 (from 0)
#10

count: 0
size: 0 bytes
in-pack: 26328
packs: 1
size-pack: 44.43 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 7.6279s

---

11 /tmp/repo-base-dir/11
Enumerating objects: 28937, done.
Counting objects: 100% (28937/28937), done.
Delta compression using up to 16 threads
Compressing objects: 100% (4726/4726), done.
Writing objects: 100% (28937/28937), done.
Building bitmaps: 100% (12/12), done.
Total 28937 (delta 26731), reused 25563 (delta 24152), pack-reused 0 (from 0)
#11

count: 0
size: 0 bytes
in-pack: 28937
packs: 1
size-pack: 47.00 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 4.8999s

---

12 /tmp/repo-base-dir/12
Enumerating objects: 31510, done.
Counting objects: 100% (31510/31510), done.
Delta compression using up to 16 threads
Compressing objects: 100% (4716/4716), done.
Writing objects: 100% (31510/31510), done.
Building bitmaps: 100% (13/13), done.
Total 31510 (delta 29261), reused 28218 (delta 26731), pack-reused 0 (from 0)
#12

count: 0
size: 0 bytes
in-pack: 31510
packs: 1
size-pack: 49.33 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 4.5425s

---

13 /tmp/repo-base-dir/13
Enumerating objects: 32759, done.
Counting objects: 100% (32759/32759), done.
Delta compression using up to 16 threads
Compressing objects: 100% (3431/3431), done.
Writing objects: 100% (32759/32759), done.
Building bitmaps: 100% (14/14), done.
Total 32759 (delta 30501), reused 31174 (delta 29261), pack-reused 0 (from 0)
#13

count: 0
size: 0 bytes
in-pack: 32759
packs: 1
size-pack: 49.97 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 0.9884s

---

## git (no terrain, git repack -a -d --depth 4095 --window 64)

0 /tmp/repo-base-dir/0
Enumerating objects: 8176, done.
Counting objects: 100% (8176/8176), done.
Delta compression using up to 16 threads
Compressing objects: 100% (8166/8166), done.
Writing objects: 100% (8176/8176), done.
Building bitmaps: 100% (1/1), done.
Total 8176 (delta 6178), reused 0 (delta 0), pack-reused 0 (from 0)
#0

count: 0
size: 0 bytes
in-pack: 8176
packs: 1
size-pack: 26.82 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 12.9461s

---

1 /tmp/repo-base-dir/1
Enumerating objects: 10902, done.
Counting objects: 100% (10902/10902), done.
Delta compression using up to 16 threads
Compressing objects: 100% (4705/4705), done.
Writing objects: 100% (10902/10902), done.
Building bitmaps: 100% (2/2), done.
Total 10902 (delta 8944), reused 7167 (delta 6178), pack-reused 0 (from 0)
#1

count: 0
size: 0 bytes
in-pack: 10902
packs: 1
size-pack: 32.62 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 3.1217s

---

2 /tmp/repo-base-dir/2
Enumerating objects: 13461, done.
Counting objects: 100% (13461/13461), done.
Delta compression using up to 16 threads
Compressing objects: 100% (4494/4494), done.
Writing objects: 100% (13461/13461), done.
Building bitmaps: 100% (3/3), done.
Total 13461 (delta 11445), reused 10072 (delta 8944), pack-reused 0 (from 0)
#2

count: 0
size: 0 bytes
in-pack: 13461
packs: 1
size-pack: 35.12 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 1.8119s

---

3 /tmp/repo-base-dir/3
Enumerating objects: 15413, done.
Counting objects: 100% (15413/15413), done.
Delta compression using up to 16 threads
Compressing objects: 100% (3941/3941), done.
Writing objects: 100% (15413/15413), done.
Building bitmaps: 100% (4/4), done.
Total 15413 (delta 13376), reused 12786 (delta 11445), pack-reused 0 (from 0)
#3

count: 0
size: 0 bytes
in-pack: 15413
packs: 1
size-pack: 36.41 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 2.4482s

---

4 /tmp/repo-base-dir/4
Enumerating objects: 16831, done.
Counting objects: 100% (16831/16831), done.
Delta compression using up to 16 threads
Compressing objects: 100% (3424/3424), done.
Writing objects: 100% (16831/16831), done.
Building bitmaps: 100% (5/5), done.
Total 16831 (delta 14775), reused 15020 (delta 13376), pack-reused 0 (from 0)
#4

count: 0
size: 0 bytes
in-pack: 16831
packs: 1
size-pack: 37.39 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 1.2038s

---

5 /tmp/repo-base-dir/5
Enumerating objects: 18060, done.
Counting objects: 100% (18060/18060), done.
Delta compression using up to 16 threads
Compressing objects: 100% (3250/3250), done.
Writing objects: 100% (18060/18060), done.
Building bitmaps: 100% (6/6), done.
Total 18060 (delta 15989), reused 16483 (delta 14775), pack-reused 0 (from 0)
#5

count: 0
size: 0 bytes
in-pack: 18060
packs: 1
size-pack: 38.17 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 0.6279s

---

6 /tmp/repo-base-dir/6
Enumerating objects: 19506, done.
Counting objects: 100% (19506/19506), done.
Delta compression using up to 16 threads
Compressing objects: 100% (3477/3477), done.
Writing objects: 100% (19506/19506), done.
Building bitmaps: 100% (7/7), done.
Total 19506 (delta 17397), reused 17521 (delta 15989), pack-reused 0 (from 0)
#6

count: 0
size: 0 bytes
in-pack: 19506
packs: 1
size-pack: 38.88 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 0.5680s

---

7 /tmp/repo-base-dir/7
Enumerating objects: 19507, done.
Counting objects: 100% (19507/19507), done.
Delta compression using up to 16 threads
Compressing objects: 100% (2070/2070), done.
Writing objects: 100% (19507/19507), done.
Building bitmaps: 100% (8/8), done.
Total 19507 (delta 17397), reused 19506 (delta 17397), pack-reused 0 (from 0)
#7

count: 0
size: 0 bytes
in-pack: 19507
packs: 1
size-pack: 38.87 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 0.1166s

---

8 /tmp/repo-base-dir/8
Enumerating objects: 21332, done.
Counting objects: 100% (21332/21332), done.
Delta compression using up to 16 threads
Compressing objects: 100% (3891/3891), done.
Writing objects: 100% (21332/21332), done.
Building bitmaps: 100% (9/9), done.
Total 21332 (delta 19179), reused 18933 (delta 17397), pack-reused 0 (from 0)
#8

count: 0
size: 0 bytes
in-pack: 21332
packs: 1
size-pack: 39.94 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 0.9594s

---

9 /tmp/repo-base-dir/9
Enumerating objects: 23706, done.
Counting objects: 100% (23706/23706), done.
Delta compression using up to 16 threads
Compressing objects: 100% (4476/4476), done.
Writing objects: 100% (23706/23706), done.
Building bitmaps: 100% (10/10), done.
Total 23706 (delta 21491), reused 20688 (delta 19179), pack-reused 0 (from 0)
#9

count: 0
size: 0 bytes
in-pack: 23706
packs: 1
size-pack: 42.21 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 2.7046s

---

10 /tmp/repo-base-dir/10
Enumerating objects: 26328, done.
Counting objects: 100% (26328/26328), done.
Delta compression using up to 16 threads
Compressing objects: 100% (4782/4782), done.
Writing objects: 100% (26328/26328), done.
Building bitmaps: 100% (11/11), done.
Total 26328 (delta 24088), reused 22885 (delta 21491), pack-reused 0 (from 0)
#10

count: 0
size: 0 bytes
in-pack: 26328
packs: 1
size-pack: 44.40 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 2.4072s

---

11 /tmp/repo-base-dir/11
Enumerating objects: 28937, done.
Counting objects: 100% (28937/28937), done.
Delta compression using up to 16 threads
Compressing objects: 100% (4790/4790), done.
Writing objects: 100% (28937/28937), done.
Building bitmaps: 100% (12/12), done.
Total 28937 (delta 26659), reused 25569 (delta 24088), pack-reused 0 (from 0)
#11

count: 0
size: 0 bytes
in-pack: 28937
packs: 1
size-pack: 46.97 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 3.1877s

---

12 /tmp/repo-base-dir/12
Enumerating objects: 31510, done.
Counting objects: 100% (31510/31510), done.
Delta compression using up to 16 threads
Compressing objects: 100% (4788/4788), done.
Writing objects: 100% (31510/31510), done.
Building bitmaps: 100% (13/13), done.
Total 31510 (delta 29181), reused 28222 (delta 26659), pack-reused 0 (from 0)
#12

count: 0
size: 0 bytes
in-pack: 31510
packs: 1
size-pack: 49.30 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 2.9427s

---

13 /tmp/repo-base-dir/13
Enumerating objects: 32759, done.
Counting objects: 100% (32759/32759), done.
Delta compression using up to 16 threads
Compressing objects: 100% (3511/3511), done.
Writing objects: 100% (32759/32759), done.
Building bitmaps: 100% (14/14), done.
Total 32759 (delta 30412), reused 31179 (delta 29181), pack-reused 0 (from 0)
#13

count: 0
size: 0 bytes
in-pack: 32759
packs: 1
size-pack: 49.95 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 0.9852s

---

## git (no terrain, git repack -a -d --depth 4095 --window 32)

0 /tmp/repo-base-dir/0
Enumerating objects: 8176, done.
Counting objects: 100% (8176/8176), done.
Delta compression using up to 16 threads
Compressing objects: 100% (8166/8166), done.
Writing objects: 100% (8176/8176), done.
Building bitmaps: 100% (1/1), done.
Total 8176 (delta 6085), reused 0 (delta 0), pack-reused 0 (from 0)
#0

count: 0
size: 0 bytes
in-pack: 8176
packs: 1
size-pack: 26.83 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 8.5866s

---

1 /tmp/repo-base-dir/1
Enumerating objects: 10902, done.
Counting objects: 100% (10902/10902), done.
Delta compression using up to 16 threads
Compressing objects: 100% (4798/4798), done.
Writing objects: 100% (10902/10902), done.
Building bitmaps: 100% (2/2), done.
Total 10902 (delta 8825), reused 7143 (delta 6085), pack-reused 0 (from 0)
#1

count: 0
size: 0 bytes
in-pack: 10902
packs: 1
size-pack: 32.62 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 2.0977s

---

2 /tmp/repo-base-dir/2
Enumerating objects: 13461, done.
Counting objects: 100% (13461/13461), done.
Delta compression using up to 16 threads
Compressing objects: 100% (4613/4613), done.
Writing objects: 100% (13461/13461), done.
Building bitmaps: 100% (3/3), done.
Total 13461 (delta 11326), reused 10044 (delta 8825), pack-reused 0 (from 0)
#2

count: 0
size: 0 bytes
in-pack: 13461
packs: 1
size-pack: 35.10 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 1.2526s

---

3 /tmp/repo-base-dir/3
Enumerating objects: 15413, done.
Counting objects: 100% (15413/15413), done.
Delta compression using up to 16 threads
Compressing objects: 100% (4060/4060), done.
Writing objects: 100% (15413/15413), done.
Building bitmaps: 100% (4/4), done.
Total 15413 (delta 13256), reused 12779 (delta 11326), pack-reused 0 (from 0)
#3

count: 0
size: 0 bytes
in-pack: 15413
packs: 1
size-pack: 36.37 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 1.0080s

---

4 /tmp/repo-base-dir/4
Enumerating objects: 16831, done.
Counting objects: 100% (16831/16831), done.
Delta compression using up to 16 threads
Compressing objects: 100% (3544/3544), done.
Writing objects: 100% (16831/16831), done.
Building bitmaps: 100% (5/5), done.
Total 16831 (delta 14656), reused 15011 (delta 13256), pack-reused 0 (from 0)
#4

count: 0
size: 0 bytes
in-pack: 16831
packs: 1
size-pack: 37.33 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 0.8855s

---

5 /tmp/repo-base-dir/5
Enumerating objects: 18060, done.
Counting objects: 100% (18060/18060), done.
Delta compression using up to 16 threads
Compressing objects: 100% (3369/3369), done.
Writing objects: 100% (18060/18060), done.
Building bitmaps: 100% (6/6), done.
Total 18060 (delta 15866), reused 16479 (delta 14656), pack-reused 0 (from 0)
#5

count: 0
size: 0 bytes
in-pack: 18060
packs: 1
size-pack: 38.10 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 0.6699s

---

6 /tmp/repo-base-dir/6
Enumerating objects: 19506, done.
Counting objects: 100% (19506/19506), done.
Delta compression using up to 16 threads
Compressing objects: 100% (3600/3600), done.
Writing objects: 100% (19506/19506), done.
Building bitmaps: 100% (7/7), done.
Total 19506 (delta 17277), reused 17501 (delta 15866), pack-reused 0 (from 0)
#6

count: 0
size: 0 bytes
in-pack: 19506
packs: 1
size-pack: 38.80 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 0.5891s

---

7 /tmp/repo-base-dir/7
Enumerating objects: 19507, done.
Counting objects: 100% (19507/19507), done.
Delta compression using up to 16 threads
Compressing objects: 100% (2190/2190), done.
Writing objects: 100% (19507/19507), done.
Building bitmaps: 100% (8/8), done.
Total 19507 (delta 17277), reused 19506 (delta 17277), pack-reused 0 (from 0)
#7

count: 0
size: 0 bytes
in-pack: 19507
packs: 1
size-pack: 38.79 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 0.1147s

---

8 /tmp/repo-base-dir/8
Enumerating objects: 21332, done.
Counting objects: 100% (21332/21332), done.
Delta compression using up to 16 threads
Compressing objects: 100% (4011/4011), done.
Writing objects: 100% (21332/21332), done.
Building bitmaps: 100% (9/9), done.
Total 21332 (delta 19061), reused 18921 (delta 17277), pack-reused 0 (from 0)
#8

count: 0
size: 0 bytes
in-pack: 21332
packs: 1
size-pack: 39.85 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 0.7626s

---

9 /tmp/repo-base-dir/9
Enumerating objects: 23706, done.
Counting objects: 100% (23706/23706), done.
Delta compression using up to 16 threads
Compressing objects: 100% (4594/4594), done.
Writing objects: 100% (23706/23706), done.
Building bitmaps: 100% (10/10), done.
Total 23706 (delta 21359), reused 20673 (delta 19061), pack-reused 0 (from 0)
#9

count: 0
size: 0 bytes
in-pack: 23706
packs: 1
size-pack: 42.10 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 1.1400s

---

10 /tmp/repo-base-dir/10
Enumerating objects: 26328, done.
Counting objects: 100% (26328/26328), done.
Delta compression using up to 16 threads
Compressing objects: 100% (4914/4914), done.
Writing objects: 100% (26328/26328), done.
Building bitmaps: 100% (11/11), done.
Total 26328 (delta 23956), reused 22854 (delta 21359), pack-reused 0 (from 0)
#10

count: 0
size: 0 bytes
in-pack: 26328
packs: 1
size-pack: 44.28 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 1.0469s

---

11 /tmp/repo-base-dir/11
Enumerating objects: 28937, done.
Counting objects: 100% (28937/28937), done.
Delta compression using up to 16 threads
Compressing objects: 100% (4922/4922), done.
Writing objects: 100% (28937/28937), done.
Building bitmaps: 100% (12/12), done.
Total 28937 (delta 26531), reused 25544 (delta 23956), pack-reused 0 (from 0)
#11

count: 0
size: 0 bytes
in-pack: 28937
packs: 1
size-pack: 46.82 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 1.2826s

---

12 /tmp/repo-base-dir/12
Enumerating objects: 31510, done.
Counting objects: 100% (31510/31510), done.
Delta compression using up to 16 threads
Compressing objects: 100% (4916/4916), done.
Writing objects: 100% (31510/31510), done.
Building bitmaps: 100% (13/13), done.
Total 31510 (delta 29049), reused 28197 (delta 26531), pack-reused 0 (from 0)
#12

count: 0
size: 0 bytes
in-pack: 31510
packs: 1
size-pack: 49.14 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 1.2450s

---

13 /tmp/repo-base-dir/13
Enumerating objects: 32759, done.
Counting objects: 100% (32759/32759), done.
Delta compression using up to 16 threads
Compressing objects: 100% (3643/3643), done.
Writing objects: 100% (32759/32759), done.
Building bitmaps: 100% (14/14), done.
Total 32759 (delta 30291), reused 31168 (delta 29049), pack-reused 0 (from 0)
#13

count: 0
size: 0 bytes
in-pack: 32759
packs: 1
size-pack: 49.79 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 0.8477s

---

## git (no terrain, git repack -a -d --depth 4095 --window 16)

0 /tmp/repo-base-dir/0
Enumerating objects: 8176, done.
Counting objects: 100% (8176/8176), done.
Delta compression using up to 16 threads
Compressing objects: 100% (8166/8166), done.
Writing objects: 100% (8176/8176), done.
Building bitmaps: 100% (1/1), done.
Total 8176 (delta 5934), reused 0 (delta 0), pack-reused 0 (from 0)
#0

count: 0
size: 0 bytes
in-pack: 8176
packs: 1
size-pack: 26.85 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 6.2824s

---

1 /tmp/repo-base-dir/1
Enumerating objects: 10902, done.
Counting objects: 100% (10902/10902), done.
Delta compression using up to 16 threads
Compressing objects: 100% (4949/4949), done.
Writing objects: 100% (10902/10902), done.
Building bitmaps: 100% (2/2), done.
Total 10902 (delta 8641), reused 7098 (delta 5934), pack-reused 0 (from 0)
#1

count: 0
size: 0 bytes
in-pack: 10902
packs: 1
size-pack: 32.63 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 1.5313s

---

2 /tmp/repo-base-dir/2
Enumerating objects: 13461, done.
Counting objects: 100% (13461/13461), done.
Delta compression using up to 16 threads
Compressing objects: 100% (4797/4797), done.
Writing objects: 100% (13461/13461), done.
Building bitmaps: 100% (3/3), done.
Total 13461 (delta 11151), reused 10012 (delta 8641), pack-reused 0 (from 0)
#2

count: 0
size: 0 bytes
in-pack: 13461
packs: 1
size-pack: 35.11 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 0.8224s

---

3 /tmp/repo-base-dir/3
Enumerating objects: 15413, done.
Counting objects: 100% (15413/15413), done.
Delta compression using up to 16 threads
Compressing objects: 100% (4235/4235), done.
Writing objects: 100% (15413/15413), done.
Building bitmaps: 100% (4/4), done.
Total 15413 (delta 13076), reused 12768 (delta 11151), pack-reused 0 (from 0)
#3

count: 0
size: 0 bytes
in-pack: 15413
packs: 1
size-pack: 36.39 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 0.5563s

---

4 /tmp/repo-base-dir/4
Enumerating objects: 16831, done.
Counting objects: 100% (16831/16831), done.
Delta compression using up to 16 threads
Compressing objects: 100% (3724/3724), done.
Writing objects: 100% (16831/16831), done.
Building bitmaps: 100% (5/5), done.
Total 16831 (delta 14483), reused 14990 (delta 13076), pack-reused 0 (from 0)
#4

count: 0
size: 0 bytes
in-pack: 16831
packs: 1
size-pack: 37.34 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 0.4798s

---

5 /tmp/repo-base-dir/5
Enumerating objects: 18060, done.
Counting objects: 100% (18060/18060), done.
Delta compression using up to 16 threads
Compressing objects: 100% (3542/3542), done.
Writing objects: 100% (18060/18060), done.
Building bitmaps: 100% (6/6), done.
Total 18060 (delta 15689), reused 16475 (delta 14483), pack-reused 0 (from 0)
#5

count: 0
size: 0 bytes
in-pack: 18060
packs: 1
size-pack: 38.10 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 0.4272s

---

6 /tmp/repo-base-dir/6
Enumerating objects: 19506, done.
Counting objects: 100% (19506/19506), done.
Delta compression using up to 16 threads
Compressing objects: 100% (3777/3777), done.
Writing objects: 100% (19506/19506), done.
Building bitmaps: 100% (7/7), done.
Total 19506 (delta 17092), reused 17488 (delta 15689), pack-reused 0 (from 0)
#6

count: 0
size: 0 bytes
in-pack: 19506
packs: 1
size-pack: 38.79 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 0.4533s

---

7 /tmp/repo-base-dir/7
Enumerating objects: 19507, done.
Counting objects: 100% (19507/19507), done.
Delta compression using up to 16 threads
Compressing objects: 100% (2375/2375), done.
Writing objects: 100% (19507/19507), done.
Building bitmaps: 100% (8/8), done.
Total 19507 (delta 17092), reused 19506 (delta 17092), pack-reused 0 (from 0)
#7

count: 0
size: 0 bytes
in-pack: 19507
packs: 1
size-pack: 38.79 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 0.1132s

---

8 /tmp/repo-base-dir/8
Enumerating objects: 21332, done.
Counting objects: 100% (21332/21332), done.
Delta compression using up to 16 threads
Compressing objects: 100% (4196/4196), done.
Writing objects: 100% (21332/21332), done.
Building bitmaps: 100% (9/9), done.
Total 21332 (delta 18891), reused 18904 (delta 17092), pack-reused 0 (from 0)
#8

count: 0
size: 0 bytes
in-pack: 21332
packs: 1
size-pack: 39.82 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 0.5512s

---

9 /tmp/repo-base-dir/9
Enumerating objects: 23706, done.
Counting objects: 100% (23706/23706), done.
Delta compression using up to 16 threads
Compressing objects: 100% (4764/4764), done.
Writing objects: 100% (23706/23706), done.
Building bitmaps: 100% (10/10), done.
Total 23706 (delta 21192), reused 20664 (delta 18891), pack-reused 0 (from 0)
#9

count: 0
size: 0 bytes
in-pack: 23706
packs: 1
size-pack: 42.05 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 0.7019s

---

10 /tmp/repo-base-dir/10
Enumerating objects: 26328, done.
Counting objects: 100% (26328/26328), done.
Delta compression using up to 16 threads
Compressing objects: 100% (5081/5081), done.
Writing objects: 100% (26328/26328), done.
Building bitmaps: 100% (11/11), done.
Total 26328 (delta 23754), reused 22855 (delta 21192), pack-reused 0 (from 0)
#10

count: 0
size: 0 bytes
in-pack: 26328
packs: 1
size-pack: 44.22 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 0.6880s

---

11 /tmp/repo-base-dir/11
Enumerating objects: 28937, done.
Counting objects: 100% (28937/28937), done.
Delta compression using up to 16 threads
Compressing objects: 100% (5124/5124), done.
Writing objects: 100% (28937/28937), done.
Building bitmaps: 100% (12/12), done.
Total 28937 (delta 26319), reused 25521 (delta 23754), pack-reused 0 (from 0)
#11

count: 0
size: 0 bytes
in-pack: 28937
packs: 1
size-pack: 46.73 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 0.8379s

---

12 /tmp/repo-base-dir/12
Enumerating objects: 31510, done.
Counting objects: 100% (31510/31510), done.
Delta compression using up to 16 threads
Compressing objects: 100% (5128/5128), done.
Writing objects: 100% (31510/31510), done.
Building bitmaps: 100% (13/13), done.
Total 31510 (delta 28846), reused 28174 (delta 26319), pack-reused 0 (from 0)
#12

count: 0
size: 0 bytes
in-pack: 31510
packs: 1
size-pack: 49.01 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 0.7961s

---

13 /tmp/repo-base-dir/13
Enumerating objects: 32759, done.
Counting objects: 100% (32759/32759), done.
Delta compression using up to 16 threads
Compressing objects: 100% (3846/3846), done.
Writing objects: 100% (32759/32759), done.
Building bitmaps: 100% (14/14), done.
Total 32759 (delta 30084), reused 31160 (delta 28846), pack-reused 0 (from 0)
#13

count: 0
size: 0 bytes
in-pack: 32759
packs: 1
size-pack: 49.64 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 0.4083s

---

## git (no terrain, git repack -a -d --depth 4095 --window 8)

0 /tmp/repo-base-dir/0
Enumerating objects: 8176, done.
Counting objects: 100% (8176/8176), done.
Delta compression using up to 16 threads
Compressing objects: 100% (8166/8166), done.
Writing objects: 100% (8176/8176), done.
Building bitmaps: 100% (1/1), done.
Total 8176 (delta 5657), reused 0 (delta 0), pack-reused 0 (from 0)
#0

count: 0
size: 0 bytes
in-pack: 8176
packs: 1
size-pack: 26.82 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 5.0912s

---

1 /tmp/repo-base-dir/1
Enumerating objects: 10902, done.
Counting objects: 100% (10902/10902), done.
Delta compression using up to 16 threads
Compressing objects: 100% (5226/5226), done.
Writing objects: 100% (10902/10902), done.
Building bitmaps: 100% (2/2), done.
Total 10902 (delta 8244), reused 7025 (delta 5657), pack-reused 0 (from 0)
#1

count: 0
size: 0 bytes
in-pack: 10902
packs: 1
size-pack: 32.57 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 1.3177s

---

2 /tmp/repo-base-dir/2
Enumerating objects: 13461, done.
Counting objects: 100% (13461/13461), done.
Delta compression using up to 16 threads
Compressing objects: 100% (5194/5194), done.
Writing objects: 100% (13461/13461), done.
Building bitmaps: 100% (3/3), done.
Total 13461 (delta 10699), reused 9948 (delta 8244), pack-reused 0 (from 0)
#2

count: 0
size: 0 bytes
in-pack: 13461
packs: 1
size-pack: 35.02 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 0.6746s

---

3 /tmp/repo-base-dir/3
Enumerating objects: 15413, done.
Counting objects: 100% (15413/15413), done.
Delta compression using up to 16 threads
Compressing objects: 100% (4687/4687), done.
Writing objects: 100% (15413/15413), done.
Building bitmaps: 100% (4/4), done.
Total 15413 (delta 12616), reused 12703 (delta 10699), pack-reused 0 (from 0)
#3

count: 0
size: 0 bytes
in-pack: 15413
packs: 1
size-pack: 36.29 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 0.4488s

---

4 /tmp/repo-base-dir/4
Enumerating objects: 16831, done.
Counting objects: 100% (16831/16831), done.
Delta compression using up to 16 threads
Compressing objects: 100% (4184/4184), done.
Writing objects: 100% (16831/16831), done.
Building bitmaps: 100% (5/5), done.
Total 16831 (delta 14024), reused 14943 (delta 12616), pack-reused 0 (from 0)
#4

count: 0
size: 0 bytes
in-pack: 16831
packs: 1
size-pack: 37.22 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 0.3551s

---

5 /tmp/repo-base-dir/5
Enumerating objects: 18060, done.
Counting objects: 100% (18060/18060), done.
Delta compression using up to 16 threads
Compressing objects: 100% (4001/4001), done.
Writing objects: 100% (18060/18060), done.
Building bitmaps: 100% (6/6), done.
Total 18060 (delta 15233), reused 16433 (delta 14024), pack-reused 0 (from 0)
#5

count: 0
size: 0 bytes
in-pack: 18060
packs: 1
size-pack: 37.96 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 0.3238s

---

6 /tmp/repo-base-dir/6
Enumerating objects: 19506, done.
Counting objects: 100% (19506/19506), done.
Delta compression using up to 16 threads
Compressing objects: 100% (4233/4233), done.
Writing objects: 100% (19506/19506), done.
Building bitmaps: 100% (7/7), done.
Total 19506 (delta 16620), reused 17456 (delta 15233), pack-reused 0 (from 0)
#6

count: 0
size: 0 bytes
in-pack: 19506
packs: 1
size-pack: 38.64 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 0.3209s

---

7 /tmp/repo-base-dir/7
Enumerating objects: 19507, done.
Counting objects: 100% (19507/19507), done.
Delta compression using up to 16 threads
Compressing objects: 100% (2847/2847), done.
Writing objects: 100% (19507/19507), done.
Building bitmaps: 100% (8/8), done.
Total 19507 (delta 16620), reused 19506 (delta 16620), pack-reused 0 (from 0)
#7

count: 0
size: 0 bytes
in-pack: 19507
packs: 1
size-pack: 38.63 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 0.1144s

---

8 /tmp/repo-base-dir/8
Enumerating objects: 21332, done.
Counting objects: 100% (21332/21332), done.
Delta compression using up to 16 threads
Compressing objects: 100% (4668/4668), done.
Writing objects: 100% (21332/21332), done.
Building bitmaps: 100% (9/9), done.
Total 21332 (delta 18407), reused 18841 (delta 16620), pack-reused 0 (from 0)
#8

count: 0
size: 0 bytes
in-pack: 21332
packs: 1
size-pack: 39.66 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 0.3852s

---

9 /tmp/repo-base-dir/9
Enumerating objects: 23706, done.
Counting objects: 100% (23706/23706), done.
Delta compression using up to 16 threads
Compressing objects: 100% (5248/5248), done.
Writing objects: 100% (23706/23706), done.
Building bitmaps: 100% (10/10), done.
Total 23706 (delta 20716), reused 20575 (delta 18407), pack-reused 0 (from 0)
#9

count: 0
size: 0 bytes
in-pack: 23706
packs: 1
size-pack: 41.85 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 0.6142s

---

10 /tmp/repo-base-dir/10
Enumerating objects: 26328, done.
Counting objects: 100% (26328/26328), done.
Delta compression using up to 16 threads
Compressing objects: 100% (5557/5557), done.
Writing objects: 100% (26328/26328), done.
Building bitmaps: 100% (11/11), done.
Total 26328 (delta 23250), reused 22791 (delta 20716), pack-reused 0 (from 0)
#10

count: 0
size: 0 bytes
in-pack: 26328
packs: 1
size-pack: 44.00 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 0.6084s

---

11 /tmp/repo-base-dir/11
Enumerating objects: 28937, done.
Counting objects: 100% (28937/28937), done.
Delta compression using up to 16 threads
Compressing objects: 100% (5628/5628), done.
Writing objects: 100% (28937/28937), done.
Building bitmaps: 100% (12/12), done.
Total 28937 (delta 25791), reused 25459 (delta 23250), pack-reused 0 (from 0)
#11

count: 0
size: 0 bytes
in-pack: 28937
packs: 1
size-pack: 46.47 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 0.6815s

---

12 /tmp/repo-base-dir/12
Enumerating objects: 31510, done.
Counting objects: 100% (31510/31510), done.
Delta compression using up to 16 threads
Compressing objects: 100% (5656/5656), done.
Writing objects: 100% (31510/31510), done.
Building bitmaps: 100% (13/13), done.
Total 31510 (delta 28304), reused 28105 (delta 25791), pack-reused 0 (from 0)
#12

count: 0
size: 0 bytes
in-pack: 31510
packs: 1
size-pack: 48.72 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 0.6494s

---

13 /tmp/repo-base-dir/13
Enumerating objects: 32759, done.
Counting objects: 100% (32759/32759), done.
Delta compression using up to 16 threads
Compressing objects: 100% (4388/4388), done.
Writing objects: 100% (32759/32759), done.
Building bitmaps: 100% (14/14), done.
Total 32759 (delta 29542), reused 31125 (delta 28304), pack-reused 0 (from 0)
#13

count: 0
size: 0 bytes
in-pack: 32759
packs: 1
size-pack: 49.35 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 0.3653s

---

## git (no terrain, git repack -a -d --depth 4095 --window 4)

0 /tmp/repo-base-dir/0
Enumerating objects: 8176, done.
Counting objects: 100% (8176/8176), done.
Delta compression using up to 16 threads
Compressing objects: 100% (8166/8166), done.
Writing objects: 100% (8176/8176), done.
Building bitmaps: 100% (1/1), done.
Total 8176 (delta 5339), reused 0 (delta 0), pack-reused 0 (from 0)
#0

count: 0
size: 0 bytes
in-pack: 8176
packs: 1
size-pack: 26.83 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 4.4763s

---

1 /tmp/repo-base-dir/1
Enumerating objects: 10902, done.
Counting objects: 100% (10902/10902), done.
Delta compression using up to 16 threads
Compressing objects: 100% (5544/5544), done.
Writing objects: 100% (10902/10902), done.
Building bitmaps: 100% (2/2), done.
Total 10902 (delta 7801), reused 6965 (delta 5339), pack-reused 0 (from 0)
#1

count: 0
size: 0 bytes
in-pack: 10902
packs: 1
size-pack: 32.54 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 1.2038s

---

2 /tmp/repo-base-dir/2
Enumerating objects: 13461, done.
Counting objects: 100% (13461/13461), done.
Delta compression using up to 16 threads
Compressing objects: 100% (5637/5637), done.
Writing objects: 100% (13461/13461), done.
Building bitmaps: 100% (3/3), done.
Total 13461 (delta 10214), reused 9830 (delta 7801), pack-reused 0 (from 0)
#2

count: 0
size: 0 bytes
in-pack: 13461
packs: 1
size-pack: 34.97 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 0.6355s

---

3 /tmp/repo-base-dir/3
Enumerating objects: 15413, done.
Counting objects: 100% (15413/15413), done.
Delta compression using up to 16 threads
Compressing objects: 100% (5172/5172), done.
Writing objects: 100% (15413/15413), done.
Building bitmaps: 100% (4/4), done.
Total 15413 (delta 12123), reused 12604 (delta 10214), pack-reused 0 (from 0)
#3

count: 0
size: 0 bytes
in-pack: 15413
packs: 1
size-pack: 36.20 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 0.4016s

---

4 /tmp/repo-base-dir/4
Enumerating objects: 16831, done.
Counting objects: 100% (16831/16831), done.
Delta compression using up to 16 threads
Compressing objects: 100% (4677/4677), done.
Writing objects: 100% (16831/16831), done.
Building bitmaps: 100% (5/5), done.
Total 16831 (delta 13539), reused 14861 (delta 12123), pack-reused 0 (from 0)
#4

count: 0
size: 0 bytes
in-pack: 16831
packs: 1
size-pack: 37.11 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 0.3366s

---

5 /tmp/repo-base-dir/5
Enumerating objects: 18060, done.
Counting objects: 100% (18060/18060), done.
Delta compression using up to 16 threads
Compressing objects: 100% (4486/4486), done.
Writing objects: 100% (18060/18060), done.
Building bitmaps: 100% (6/6), done.
Total 18060 (delta 14732), reused 16392 (delta 13539), pack-reused 0 (from 0)
#5

count: 0
size: 0 bytes
in-pack: 18060
packs: 1
size-pack: 37.86 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 0.3015s

---

6 /tmp/repo-base-dir/6
Enumerating objects: 19506, done.
Counting objects: 100% (19506/19506), done.
Delta compression using up to 16 threads
Compressing objects: 100% (4734/4734), done.
Writing objects: 100% (19506/19506), done.
Building bitmaps: 100% (7/7), done.
Total 19506 (delta 16101), reused 17379 (delta 14732), pack-reused 0 (from 0)
#6

count: 0
size: 0 bytes
in-pack: 19506
packs: 1
size-pack: 38.53 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 0.2981s

---

7 /tmp/repo-base-dir/7
Enumerating objects: 19507, done.
Counting objects: 100% (19507/19507), done.
Delta compression using up to 16 threads
Compressing objects: 100% (3366/3366), done.
Writing objects: 100% (19507/19507), done.
Building bitmaps: 100% (8/8), done.
Total 19507 (delta 16101), reused 19506 (delta 16101), pack-reused 0 (from 0)
#7

count: 0
size: 0 bytes
in-pack: 19507
packs: 1
size-pack: 38.52 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 0.1116s

---

8 /tmp/repo-base-dir/8
Enumerating objects: 21332, done.
Counting objects: 100% (21332/21332), done.
Delta compression using up to 16 threads
Compressing objects: 100% (5187/5187), done.
Writing objects: 100% (21332/21332), done.
Building bitmaps: 100% (9/9), done.
Total 21332 (delta 17887), reused 18782 (delta 16101), pack-reused 0 (from 0)
#8

count: 0
size: 0 bytes
in-pack: 21332
packs: 1
size-pack: 39.51 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 0.3503s

---

9 /tmp/repo-base-dir/9
Enumerating objects: 23706, done.
Counting objects: 100% (23706/23706), done.
Delta compression using up to 16 threads
Compressing objects: 100% (5768/5768), done.
Writing objects: 100% (23706/23706), done.
Building bitmaps: 100% (10/10), done.
Total 23706 (delta 20170), reused 20492 (delta 17887), pack-reused 0 (from 0)
#9

count: 0
size: 0 bytes
in-pack: 23706
packs: 1
size-pack: 41.67 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 0.5734s

---

10 /tmp/repo-base-dir/10
Enumerating objects: 26328, done.
Counting objects: 100% (26328/26328), done.
Delta compression using up to 16 threads
Compressing objects: 100% (6103/6103), done.
Writing objects: 100% (26328/26328), done.
Building bitmaps: 100% (11/11), done.
Total 26328 (delta 22687), reused 22678 (delta 20170), pack-reused 0 (from 0)
#10

count: 0
size: 0 bytes
in-pack: 26328
packs: 1
size-pack: 43.79 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 0.5789s

---

11 /tmp/repo-base-dir/11
Enumerating objects: 28937, done.
Counting objects: 100% (28937/28937), done.
Delta compression using up to 16 threads
Compressing objects: 100% (6191/6191), done.
Writing objects: 100% (28937/28937), done.
Building bitmaps: 100% (12/12), done.
Total 28937 (delta 25208), reused 25352 (delta 22687), pack-reused 0 (from 0)
#11

count: 0
size: 0 bytes
in-pack: 28937
packs: 1
size-pack: 46.21 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 0.6229s

---

12 /tmp/repo-base-dir/12
Enumerating objects: 31510, done.
Counting objects: 100% (31510/31510), done.
Delta compression using up to 16 threads
Compressing objects: 100% (6239/6239), done.
Writing objects: 100% (31510/31510), done.
Building bitmaps: 100% (13/13), done.
Total 31510 (delta 27726), reused 27977 (delta 25208), pack-reused 0 (from 0)
#12

count: 0
size: 0 bytes
in-pack: 31510
packs: 1
size-pack: 48.41 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 0.6093s

---

13 /tmp/repo-base-dir/13
Enumerating objects: 32759, done.
Counting objects: 100% (32759/32759), done.
Delta compression using up to 16 threads
Compressing objects: 100% (4966/4966), done.
Writing objects: 100% (32759/32759), done.
Building bitmaps: 100% (14/14), done.
Total 32759 (delta 28972), reused 31079 (delta 27726), pack-reused 0 (from 0)
#13

count: 0
size: 0 bytes
in-pack: 32759
packs: 1
size-pack: 49.02 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 0.3327s

---

## git (no terrain, git repack -a -d --depth 4095 --window 2)

0 /tmp/repo-base-dir/0
Enumerating objects: 8176, done.
Counting objects: 100% (8176/8176), done.
Delta compression using up to 16 threads
Compressing objects: 100% (8166/8166), done.
Writing objects: 100% (8176/8176), done.
Building bitmaps: 100% (1/1), done.
Total 8176 (delta 4949), reused 0 (delta 0), pack-reused 0 (from 0)
#0

count: 0
size: 0 bytes
in-pack: 8176
packs: 1
size-pack: 26.72 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 4.1603s

---

1 /tmp/repo-base-dir/1
Enumerating objects: 10902, done.
Counting objects: 100% (10902/10902), done.
Delta compression using up to 16 threads
Compressing objects: 100% (5934/5934), done.
Writing objects: 100% (10902/10902), done.
Building bitmaps: 100% (2/2), done.
Total 10902 (delta 7203), reused 7061 (delta 4949), pack-reused 0 (from 0)
#1

count: 0
size: 0 bytes
in-pack: 10902
packs: 1
size-pack: 32.50 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 1.1262s

---

2 /tmp/repo-base-dir/2
Enumerating objects: 13461, done.
Counting objects: 100% (13461/13461), done.
Delta compression using up to 16 threads
Compressing objects: 100% (6235/6235), done.
Writing objects: 100% (13461/13461), done.
Building bitmaps: 100% (3/3), done.
Total 13461 (delta 9530), reused 9688 (delta 7203), pack-reused 0 (from 0)
#2

count: 0
size: 0 bytes
in-pack: 13461
packs: 1
size-pack: 34.91 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 0.6177s

---

3 /tmp/repo-base-dir/3
Enumerating objects: 15413, done.
Counting objects: 100% (15413/15413), done.
Delta compression using up to 16 threads
Compressing objects: 100% (5856/5856), done.
Writing objects: 100% (15413/15413), done.
Building bitmaps: 100% (4/4), done.
Total 15413 (delta 11356), reused 12525 (delta 9530), pack-reused 0 (from 0)
#3

count: 0
size: 0 bytes
in-pack: 15413
packs: 1
size-pack: 36.12 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 0.3955s

---

4 /tmp/repo-base-dir/4
Enumerating objects: 16831, done.
Counting objects: 100% (16831/16831), done.
Delta compression using up to 16 threads
Compressing objects: 100% (5444/5444), done.
Writing objects: 100% (16831/16831), done.
Building bitmaps: 100% (5/5), done.
Total 16831 (delta 12739), reused 14752 (delta 11356), pack-reused 0 (from 0)
#4

count: 0
size: 0 bytes
in-pack: 16831
packs: 1
size-pack: 36.98 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 0.3214s

---

5 /tmp/repo-base-dir/5
Enumerating objects: 18060, done.
Counting objects: 100% (18060/18060), done.
Delta compression using up to 16 threads
Compressing objects: 100% (5286/5286), done.
Writing objects: 100% (18060/18060), done.
Building bitmaps: 100% (6/6), done.
Total 18060 (delta 13936), reused 16276 (delta 12739), pack-reused 0 (from 0)
#5

count: 0
size: 0 bytes
in-pack: 18060
packs: 1
size-pack: 37.68 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 0.2923s

---

6 /tmp/repo-base-dir/6
Enumerating objects: 19506, done.
Counting objects: 100% (19506/19506), done.
Delta compression using up to 16 threads
Compressing objects: 100% (5530/5530), done.
Writing objects: 100% (19506/19506), done.
Building bitmaps: 100% (7/7), done.
Total 19506 (delta 15289), reused 17317 (delta 13936), pack-reused 0 (from 0)
#6

count: 0
size: 0 bytes
in-pack: 19506
packs: 1
size-pack: 38.32 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 0.2851s

---

7 /tmp/repo-base-dir/7
Enumerating objects: 19507, done.
Counting objects: 100% (19507/19507), done.
Delta compression using up to 16 threads
Compressing objects: 100% (4178/4178), done.
Writing objects: 100% (19507/19507), done.
Building bitmaps: 100% (8/8), done.
Total 19507 (delta 15289), reused 19506 (delta 15289), pack-reused 0 (from 0)
#7

count: 0
size: 0 bytes
in-pack: 19507
packs: 1
size-pack: 38.31 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 0.1140s

---

8 /tmp/repo-base-dir/8
Enumerating objects: 21332, done.
Counting objects: 100% (21332/21332), done.
Delta compression using up to 16 threads
Compressing objects: 100% (5999/5999), done.
Writing objects: 100% (21332/21332), done.
Building bitmaps: 100% (9/9), done.
Total 21332 (delta 17049), reused 18667 (delta 15289), pack-reused 0 (from 0)
#8

count: 0
size: 0 bytes
in-pack: 21332
packs: 1
size-pack: 39.27 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 0.3441s

---

9 /tmp/repo-base-dir/9
Enumerating objects: 23706, done.
Counting objects: 100% (23706/23706), done.
Delta compression using up to 16 threads
Compressing objects: 100% (6606/6606), done.
Writing objects: 100% (23706/23706), done.
Building bitmaps: 100% (10/10), done.
Total 23706 (delta 19278), reused 20397 (delta 17049), pack-reused 0 (from 0)
#9

count: 0
size: 0 bytes
in-pack: 23706
packs: 1
size-pack: 41.38 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 0.5505s

---

10 /tmp/repo-base-dir/10
Enumerating objects: 26328, done.
Counting objects: 100% (26328/26328), done.
Delta compression using up to 16 threads
Compressing objects: 100% (6995/6995), done.
Writing objects: 100% (26328/26328), done.
Building bitmaps: 100% (11/11), done.
Total 26328 (delta 21725), reused 22553 (delta 19278), pack-reused 0 (from 0)
#10

count: 0
size: 0 bytes
in-pack: 26328
packs: 1
size-pack: 43.47 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 0.5721s

---

11 /tmp/repo-base-dir/11
Enumerating objects: 28937, done.
Counting objects: 100% (28937/28937), done.
Delta compression using up to 16 threads
Compressing objects: 100% (7153/7153), done.
Writing objects: 100% (28937/28937), done.
Building bitmaps: 100% (12/12), done.
Total 28937 (delta 24155), reused 25256 (delta 21725), pack-reused 0 (from 0)
#11

count: 0
size: 0 bytes
in-pack: 28937
packs: 1
size-pack: 45.90 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 0.6306s

---

12 /tmp/repo-base-dir/12
Enumerating objects: 31510, done.
Counting objects: 100% (31510/31510), done.
Delta compression using up to 16 threads
Compressing objects: 100% (7292/7292), done.
Writing objects: 100% (31510/31510), done.
Building bitmaps: 100% (13/13), done.
Total 31510 (delta 26564), reused 27896 (delta 24155), pack-reused 0 (from 0)
#12

count: 0
size: 0 bytes
in-pack: 31510
packs: 1
size-pack: 48.10 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 0.5994s

---

13 /tmp/repo-base-dir/13
Enumerating objects: 32759, done.
Counting objects: 100% (32759/32759), done.
Delta compression using up to 16 threads
Compressing objects: 100% (6128/6128), done.
Writing objects: 100% (32759/32759), done.
Building bitmaps: 100% (14/14), done.
Total 32759 (delta 27826), reused 30966 (delta 26564), pack-reused 0 (from 0)
#13

count: 0
size: 0 bytes
in-pack: 32759
packs: 1
size-pack: 48.67 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 0.3097s

---

## git (no terrain, git repack -a -d --depth 4095 --window 1)

0 /tmp/repo-base-dir/0
Enumerating objects: 8176, done.
Counting objects: 100% (8176/8176), done.
Delta compression using up to 16 threads
Compressing objects: 100% (8166/8166), done.
Writing objects: 100% (8176/8176), done.
Building bitmaps: 100% (1/1), done.
Total 8176 (delta 4536), reused 0 (delta 0), pack-reused 0 (from 0)
#0

count: 0
size: 0 bytes
in-pack: 8176
packs: 1
size-pack: 26.66 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 4.0949s

---

1 /tmp/repo-base-dir/1
Enumerating objects: 10902, done.
Counting objects: 100% (10902/10902), done.
Delta compression using up to 16 threads
Compressing objects: 100% (6347/6347), done.
Writing objects: 100% (10902/10902), done.
Building bitmaps: 100% (2/2), done.
Total 10902 (delta 6694), reused 7096 (delta 4536), pack-reused 0 (from 0)
#1

count: 0
size: 0 bytes
in-pack: 10902
packs: 1
size-pack: 32.43 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 1.1143s

---

2 /tmp/repo-base-dir/2
Enumerating objects: 13461, done.
Counting objects: 100% (13461/13461), done.
Delta compression using up to 16 threads
Compressing objects: 100% (6744/6744), done.
Writing objects: 100% (13461/13461), done.
Building bitmaps: 100% (3/3), done.
Total 13461 (delta 8854), reused 9681 (delta 6694), pack-reused 0 (from 0)
#2

count: 0
size: 0 bytes
in-pack: 13461
packs: 1
size-pack: 34.89 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 0.6061s

---

3 /tmp/repo-base-dir/3
Enumerating objects: 15413, done.
Counting objects: 100% (15413/15413), done.
Delta compression using up to 16 threads
Compressing objects: 100% (6532/6532), done.
Writing objects: 100% (15413/15413), done.
Building bitmaps: 100% (4/4), done.
Total 15413 (delta 10555), reused 12502 (delta 8854), pack-reused 0 (from 0)
#3

count: 0
size: 0 bytes
in-pack: 15413
packs: 1
size-pack: 36.18 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 0.3835s

---

4 /tmp/repo-base-dir/4
Enumerating objects: 16831, done.
Counting objects: 100% (16831/16831), done.
Delta compression using up to 16 threads
Compressing objects: 100% (6245/6245), done.
Writing objects: 100% (16831/16831), done.
Building bitmaps: 100% (5/5), done.
Total 16831 (delta 11903), reused 14681 (delta 10555), pack-reused 0 (from 0)
#4

count: 0
size: 0 bytes
in-pack: 16831
packs: 1
size-pack: 37.07 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 0.3172s

---

5 /tmp/repo-base-dir/5
Enumerating objects: 18060, done.
Counting objects: 100% (18060/18060), done.
Delta compression using up to 16 threads
Compressing objects: 100% (6122/6122), done.
Writing objects: 100% (18060/18060), done.
Building bitmaps: 100% (6/6), done.
Total 18060 (delta 13065), reused 16217 (delta 11903), pack-reused 0 (from 0)
#5

count: 0
size: 0 bytes
in-pack: 18060
packs: 1
size-pack: 37.80 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 0.2799s

---

6 /tmp/repo-base-dir/6
Enumerating objects: 19506, done.
Counting objects: 100% (19506/19506), done.
Delta compression using up to 16 threads
Compressing objects: 100% (6401/6401), done.
Writing objects: 100% (19506/19506), done.
Building bitmaps: 100% (7/7), done.
Total 19506 (delta 14341), reused 17308 (delta 13065), pack-reused 0 (from 0)
#6

count: 0
size: 0 bytes
in-pack: 19506
packs: 1
size-pack: 38.44 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 0.2646s

---

7 /tmp/repo-base-dir/7
Enumerating objects: 19507, done.
Counting objects: 100% (19507/19507), done.
Delta compression using up to 16 threads
Compressing objects: 100% (5126/5126), done.
Writing objects: 100% (19507/19507), done.
Building bitmaps: 100% (8/8), done.
Total 19507 (delta 14341), reused 19506 (delta 14341), pack-reused 0 (from 0)
#7

count: 0
size: 0 bytes
in-pack: 19507
packs: 1
size-pack: 38.43 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 0.1086s

---

8 /tmp/repo-base-dir/8
Enumerating objects: 21332, done.
Counting objects: 100% (21332/21332), done.
Delta compression using up to 16 threads
Compressing objects: 100% (6947/6947), done.
Writing objects: 100% (21332/21332), done.
Building bitmaps: 100% (9/9), done.
Total 21332 (delta 16024), reused 18608 (delta 14341), pack-reused 0 (from 0)
#8

count: 0
size: 0 bytes
in-pack: 21332
packs: 1
size-pack: 39.41 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 0.3358s

---

9 /tmp/repo-base-dir/9
Enumerating objects: 23706, done.
Counting objects: 100% (23706/23706), done.
Delta compression using up to 16 threads
Compressing objects: 100% (7631/7631), done.
Writing objects: 100% (23706/23706), done.
Building bitmaps: 100% (10/10), done.
Total 23706 (delta 18179), reused 20295 (delta 16024), pack-reused 0 (from 0)
#9

count: 0
size: 0 bytes
in-pack: 23706
packs: 1
size-pack: 41.55 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 0.5443s

---

10 /tmp/repo-base-dir/10
Enumerating objects: 26328, done.
Counting objects: 100% (26328/26328), done.
Delta compression using up to 16 threads
Compressing objects: 100% (8094/8094), done.
Writing objects: 100% (26328/26328), done.
Building bitmaps: 100% (11/11), done.
Total 26328 (delta 20498), reused 22501 (delta 18179), pack-reused 0 (from 0)
#10

count: 0
size: 0 bytes
in-pack: 26328
packs: 1
size-pack: 43.68 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 0.5532s

---

11 /tmp/repo-base-dir/11
Enumerating objects: 28937, done.
Counting objects: 100% (28937/28937), done.
Delta compression using up to 16 threads
Compressing objects: 100% (8380/8380), done.
Writing objects: 100% (28937/28937), done.
Building bitmaps: 100% (12/12), done.
Total 28937 (delta 22795), reused 25197 (delta 20498), pack-reused 0 (from 0)
#11

count: 0
size: 0 bytes
in-pack: 28937
packs: 1
size-pack: 46.15 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 0.6151s

---

12 /tmp/repo-base-dir/12
Enumerating objects: 31510, done.
Counting objects: 100% (31510/31510), done.
Delta compression using up to 16 threads
Compressing objects: 100% (8652/8652), done.
Writing objects: 100% (31510/31510), done.
Building bitmaps: 100% (13/13), done.
Total 31510 (delta 25073), reused 27812 (delta 22795), pack-reused 0 (from 0)
#12

count: 0
size: 0 bytes
in-pack: 31510
packs: 1
size-pack: 48.39 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 0.5810s

---

13 /tmp/repo-base-dir/13
Enumerating objects: 32759, done.
Counting objects: 100% (32759/32759), done.
Delta compression using up to 16 threads
Compressing objects: 100% (7619/7619), done.
Writing objects: 100% (32759/32759), done.
Building bitmaps: 100% (14/14), done.
Total 32759 (delta 26244), reused 30956 (delta 25073), pack-reused 0 (from 0)
#13

count: 0
size: 0 bytes
in-pack: 32759
packs: 1
size-pack: 48.96 MiB
prune-packable: 0
garbage: 0
size-garbage: 0 bytes

Executed in 0.2926s

---
