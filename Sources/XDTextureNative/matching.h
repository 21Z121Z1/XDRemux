#pragma once

// Maximum-weight rectangular assignment, with one zero-weight unmatched column
// per face. Greedy selection can consume the only valid instance of a second
// face. Here every face competes globally for distinct native instance masks.
static NSDictionary *SelectFaceInstances(NSArray<NSDictionary *> *edges, NSArray<NSString *> *roles, NSUInteger faceCount) {
    if(!faceCount || !roles.count)return @{};
    NSUInteger n=faceCount,m=roles.count+faceCount;
    NSMutableData *weightsData=[NSMutableData dataWithLength:(n+1)*(m+1)*sizeof(double)];
    double *weights=weightsData.mutableBytes;
    for(NSDictionary *edge in edges) {
        NSUInteger face=[edge[@"face_index"] unsignedLongValue]+1,column=[roles indexOfObject:edge[@"instance_role"]]+1;
        double overlap=[edge[@"face_probability_overlap"] doubleValue];
        if(face<=n && column<=roles.count && overlap>=0.02)weights[face*(m+1)+column]=overlap;
    }
    NSMutableData *ud=[NSMutableData dataWithLength:(n+1)*sizeof(double)],*vd=[NSMutableData dataWithLength:(m+1)*sizeof(double)];
    NSMutableData *pd=[NSMutableData dataWithLength:(m+1)*sizeof(NSUInteger)],*wd=[NSMutableData dataWithLength:(m+1)*sizeof(NSUInteger)];
    double *u=ud.mutableBytes,*v=vd.mutableBytes;NSUInteger *p=pd.mutableBytes,*way=wd.mutableBytes;
    for(NSUInteger face=1;face<=n;face++) {
        p[0]=face;NSUInteger column=0;
        NSMutableData *minsData=[NSMutableData dataWithLength:(m+1)*sizeof(double)],*usedData=[NSMutableData dataWithLength:(m+1)*sizeof(BOOL)];
        double *mins=minsData.mutableBytes;BOOL *used=usedData.mutableBytes;
        for(NSUInteger j=1;j<=m;j++)mins[j]=INFINITY;
        do {
            used[column]=YES;NSUInteger active=p[column],next=0;double delta=INFINITY;
            for(NSUInteger j=1;j<=m;j++)if(!used[j]) {
                double cost=-weights[active*(m+1)+j]-u[active]-v[j];
                if(cost<mins[j]){mins[j]=cost;way[j]=column;}
                if(mins[j]<delta){delta=mins[j];next=j;}
            }
            for(NSUInteger j=0;j<=m;j++) {
                if(used[j]){u[p[j]]+=delta;v[j]-=delta;}else mins[j]-=delta;
            }
            column=next;
        }while(p[column]!=0);
        do{NSUInteger previous=way[column];p[column]=p[previous];column=previous;}while(column);
    }
    NSMutableDictionary *matched=[NSMutableDictionary new];
    for(NSUInteger j=1;j<=roles.count;j++)if(p[j] && weights[p[j]*(m+1)+j]>=0.02) {
        NSNumber *face=@(p[j]-1);
        for(NSDictionary *edge in edges)if([edge[@"face_index"] isEqual:face] && [edge[@"instance_role"] isEqual:roles[j-1]]){matched[face]=edge;break;}
    }
    return matched;
}
